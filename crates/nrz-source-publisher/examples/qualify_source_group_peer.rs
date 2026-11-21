//! Manual peer for qualification against the platform NativeStore receiver.
//! Build with `cargo build --locked -p nrz-source-publisher --example qualify_source_group_peer`.
//! Run with optional positional file count and member bytes (defaults: 10240 and 64).
//! Stdout first provides endpointId/catalogFile; stdin supplies the receiver target JSON.
use nrz_source_publisher::ingress_v2::{
    FrozenTreeBlob, PublicationOutcome, TransferProvider, TransferTarget,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    io::{BufRead as _, Read as _, Write as _},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count: usize = args
        .first()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(10240);
    let size: usize = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(64);
    if count == 0 || count > u32::MAX as usize || !(8..=16384).contains(&size) {
        return Err("fixture requires count > 0 and unique 8..=16384 byte members".into());
    }
    let directory = tempfile::Builder::new()
        .prefix("nrz-source-group-")
        .tempdir()?;
    let mut catalog = Vec::with_capacity(count);
    let mut blobs = Vec::with_capacity(count);
    let started = std::time::Instant::now();
    for index in 0..count {
        let mut bytes = vec![0; size];
        for (offset, byte) in bytes.iter_mut().enumerate() {
            *byte = ((index.wrapping_mul(31) + offset.wrapping_mul(17)) % 251) as u8;
        }
        bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
        let hash = bao_tree::blake3::hash(&bytes);
        let path = format!("file-{index:08}.bin");
        std::fs::write(directory.path().join(&path), &bytes)?;
        catalog.push(json!({"path":path,"hash":hash.as_bytes(),"bytes":size}));
        blobs.push(FrozenTreeBlob {
            hash: *hash.as_bytes(),
            bytes: size as u64,
            path: path.into(),
        });
    }
    let catalog_file = directory.path().join("catalog.json");
    std::fs::write(&catalog_file, serde_json::to_vec(&catalog)?)?;
    let provider = TransferProvider::bind_frozen_tree(directory, blobs).await?;
    println!(
        "{}",
        json!({"endpointId":provider.endpoint_id(),"catalogFile":catalog_file,"fileCount":count,"totalBytes":count as u64 * size as u64,"prepareMs":started.elapsed().as_millis()})
    );
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().take(32768).read_line(&mut line)?;
    let target: Value = serde_json::from_str(&line)?;
    let string = |key: &str| -> Result<&str> {
        target
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("missing {key}").into())
    };
    let seconds = target
        .get("deadlineSecs")
        .and_then(Value::as_u64)
        .unwrap_or(120);
    if seconds == 0 || seconds > 600 {
        return Err("deadlineSecs must be 1..=600".into());
    }
    let addresses = target
        .get("addresses")
        .and_then(Value::as_array)
        .ok_or("missing addresses")?
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or("invalid address")?
                .parse()
                .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let started = std::time::Instant::now();
    let outcome = provider
        .publish_resume_with_deadlines(
            TransferTarget {
                endpoint_id: string("endpointId")?.into(),
                direct_addresses: addresses,
                operation_id: string("operationId")?.parse()?,
                session_id: string("sessionId")?.parse()?,
                manifest_blake3: string("manifestBlake3")?.into(),
            },
            string("grant")?.into(),
            deadline,
            deadline,
        )
        .await?;
    if outcome != PublicationOutcome::Published {
        return Err("publication was not confirmed".into());
    }
    println!(
        "{}",
        json!({"outcome":"Published","elapsedMs":started.elapsed().as_millis(),"fileCount":count,"totalBytes":count as u64 * size as u64})
    );
    Ok(())
}
