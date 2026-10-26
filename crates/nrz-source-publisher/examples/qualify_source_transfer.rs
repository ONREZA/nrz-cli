//! Manual qualification: cargo run --release -p nrz-source-publisher --example qualify_source_transfer
//! Measures warm local files, direct loopback suffixes and interrupted stream/connection recovery.
//! The isolated receiver exercises framing, not platform grant signatures or publication authority.

use anyhow::{Context as _, ensure};
use bao_tree::{
    BaoTree, ChunkNum, ChunkRanges,
    io::{
        BaoContentItem, outboard::PreOrderOutboard, sync::CreateOutboard,
        sync::encode_ranges_validated,
    },
};
use iroh::{
    Endpoint,
    endpoint::{Connection, presets},
};
use iroh_blobs::{Hash, get::fsm, protocol::GetRequest};
use nrz_contract::artifact_ingress_v1::{self as wire, Open, Reply, Resume};
use nrz_source_publisher::ingress_v2::{
    FrozenBlob, PublicationOutcome, TransferProvider, TransferTarget, read_frame, write_frame,
};
use std::{
    fs::File,
    io::{self, Read as _, Seek as _, SeekFrom, Write as _},
    time::Duration,
};
use tokio::time::Instant;
use uuid::Uuid;

const BYTES: u64 = 128 * 1024 * 1024;
const INTERRUPT_AT: u64 = BYTES / 4;

fn outboard(file: &mut File) -> anyhow::Result<PreOrderOutboard<File>> {
    file.seek(SeekFrom::Start(0))?;
    let mut outboard = PreOrderOutboard {
        root: [0; 32].into(),
        tree: BaoTree::new(BYTES, iroh_blobs::store::IROH_BLOCK_SIZE),
        data: tempfile::tempfile()?,
    };
    outboard.init_from(file)?;
    Ok(outboard)
}

async fn fetch(
    connection: &Connection,
    hash: Hash,
    start_offset: u64,
    stop: Option<u64>,
) -> anyhow::Result<u64> {
    let started = Instant::now();
    let before = connection.stats().udp_rx.bytes;
    let connected = fsm::start(
        connection.clone(),
        GetRequest::blob_ranges(hash, ChunkRanges::from(ChunkNum(start_offset / 1024)..)),
        Default::default(),
    )
    .next()
    .await?;
    let fsm::ConnectedNext::StartRoot(root) = connected.next().await? else {
        anyhow::bail!("expected raw root");
    };
    let (mut content, size) = root.next().next().await?;
    ensure!(size == BYTES, "unexpected blob size");
    let header_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut first_leaf_ms = None;
    let mut received = 0;
    let end = loop {
        match content.next().await {
            fsm::BlobContentNext::More((next, item)) => {
                if let BaoContentItem::Leaf(leaf) = item? {
                    first_leaf_ms.get_or_insert_with(|| started.elapsed().as_secs_f64() * 1000.0);
                    ensure!(
                        leaf.offset == start_offset + received,
                        "unexpected verified offset"
                    );
                    received += leaf.data.len() as u64;
                }
                if stop.is_some_and(|limit| received >= limit) {
                    // Dropping the reader deliberately interrupts this stream, not the connection.
                    drop(next);
                    println!(
                        "interrupted verifiedBytes={received} headerMs={header_ms:.3} firstLeafMs={:.3} elapsedMs={:.3} udpRxBytes={}",
                        first_leaf_ms.unwrap_or_default(),
                        started.elapsed().as_secs_f64() * 1000.0,
                        connection.stats().udp_rx.bytes - before
                    );
                    return Ok(received);
                }
                content = next;
            }
            fsm::BlobContentNext::Done(end) => break end,
        }
    };
    let fsm::EndBlobNext::Closing(closing) = end.next() else {
        anyhow::bail!("unexpected collection child");
    };
    let stats = closing.next().await?;
    ensure!(
        received == BYTES - start_offset,
        "incomplete verified payload"
    );
    println!(
        "complete startOffset={start_offset} verifiedBytes={received} headerMs={header_ms:.3} firstLeafMs={:.3} elapsedMs={:.3} payloadBytes={} baoBytes={} udpRxBytes={}",
        first_leaf_ms.unwrap_or_default(),
        started.elapsed().as_secs_f64() * 1000.0,
        stats.payload_bytes_read,
        stats.other_bytes_read,
        connection.stats().udp_rx.bytes - before
    );
    Ok(received)
}

async fn session(file: &File, hash: Hash, scenario: &str) -> anyhow::Result<()> {
    let receiver = Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")?
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![wire::RESUME_ALPN.to_vec()])
        .bind()
        .await?;
    let provider = TransferProvider::bind([FrozenBlob {
        hash: *hash.as_bytes(),
        bytes: BYTES,
        file: file.try_clone()?,
    }])
    .await?;
    let operation_id = Uuid::from_u128(1);
    let session_id = Uuid::from_u128(2);
    let manifest = hash.to_string();
    let target = TransferTarget {
        endpoint_id: receiver.id().to_string(),
        direct_addresses: receiver.addr().ip_addrs().copied().collect(),
        operation_id,
        session_id,
        manifest_blake3: manifest.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(90);
    let publishing = tokio::spawn(provider.publish_resume_with_deadlines(
        target,
        "qualification-grant".into(),
        deadline,
        deadline,
    ));
    let incoming = receiver.accept().await.context("incoming publisher")?;
    let mut connection = incoming.await?;
    let (mut control, mut open) = connection.accept_bi().await?;
    let opening: Open = read_frame(&mut open, wire::MAX_OPEN_BYTES).await?;
    ensure!(opening.transfer_grant == "qualification-grant");
    write_frame(
        &mut control,
        &Reply::Accepted {
            operation_id,
            session_id,
        },
        wire::MAX_REPLY_BYTES,
    )
    .await?;
    println!("scenario={scenario}");
    match scenario {
        "repeat" => {
            for iteration in 1..=3 {
                print!("iteration={iteration} ");
                fetch(&connection, hash, 0, None).await?;
            }
        }
        "range" => {
            fetch(&connection, hash, INTERRUPT_AT, None).await?;
        }
        "interrupt" | "reconnect" => {
            let received = fetch(&connection, hash, 0, Some(INTERRUPT_AT)).await?;
            ensure!(received == INTERRUPT_AT);
            if scenario == "reconnect" {
                let identity = connection.remote_id();
                connection.close(0u32.into(), b"qualification injected disconnect");
                connection = receiver
                    .accept()
                    .await
                    .context("resuming publisher")?
                    .await?;
                ensure!(
                    connection.remote_id() == identity,
                    "provider identity changed"
                );
                let (send, mut receive) = connection.accept_bi().await?;
                control = send;
                let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES).await?;
                ensure!(request.operation_id == operation_id && request.session_id == session_id);
                write_frame(
                    &mut control,
                    &Reply::Accepted {
                        operation_id,
                        session_id,
                    },
                    wire::MAX_REPLY_BYTES,
                )
                .await?;
            }
            let suffix = fetch(&connection, hash, received, None).await?;
            ensure!(received + suffix == BYTES);
            println!("recovered=true retransmittedPrefixBytes=0");
        }
        _ => unreachable!(),
    }
    write_frame(
        &mut control,
        &Reply::Published {
            operation_id,
            session_id,
            manifest_blake3: manifest,
        },
        wire::MAX_REPLY_BYTES,
    )
    .await?;
    control.finish()?;
    ensure!(publishing.await?? == PublicationOutcome::Published);
    receiver.close().await;
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let mut file = tempfile::tempfile_in("/var/tmp")?;
    let block: Vec<u8> = (0..1024 * 1024)
        .map(|index| ((index * 31 + index / 251) % 256) as u8)
        .collect();
    for _ in 0..BYTES / block.len() as u64 {
        file.write_all(&block)?;
    }
    file.flush()?;
    let expected = outboard(&mut file)?.root;
    println!("fixtureBytes={BYTES} blake3={expected} cache=warm network=direct-loopback");
    for iteration in 1..=3 {
        let mut buffer = vec![0; 1024 * 1024];
        file.seek(SeekFrom::Start(0))?;
        let started = Instant::now();
        let mut raw = 0;
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            raw += read as u64;
        }
        ensure!(raw == BYTES);
        let raw_ms = started.elapsed().as_secs_f64() * 1000.0;
        file.seek(SeekFrom::Start(0))?;
        let started = Instant::now();
        let mut hasher = bao_tree::blake3::Hasher::new();
        hasher.update_reader(&mut file)?;
        ensure!(hasher.finalize() == expected);
        let hash_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let outboard = outboard(&mut file)?;
        ensure!(outboard.root == expected);
        let outboard_ms = started.elapsed().as_secs_f64() * 1000.0;
        let outboard_bytes = outboard.data.metadata()?.len();
        let started = Instant::now();
        encode_ranges_validated(&file, &outboard, &ChunkRanges::all(), io::sink())?;
        let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "local iteration={iteration} rawReadMs={raw_ms:.3} hashMs={hash_ms:.3} outboardMs={outboard_ms:.3} outboardBytes={outboard_bytes} encodeValidatedMs={encode_ms:.3}"
        );
    }
    let hash = Hash::from(*expected.as_bytes());
    for scenario in ["repeat", "range", "interrupt", "reconnect"] {
        tokio::time::timeout(Duration::from_secs(100), session(&file, hash, scenario)).await??;
    }
    Ok(())
}

#[cfg(test)]
#[path = "qualify_source_transfer/tests.rs"]
mod tests;
