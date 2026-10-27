//! Ephemeral provider for one frozen Source V2 operation and absolute lifetime.
use anyhow::{Context as _, ensure};
use bao_tree::{
    BaoTree, ChunkRanges,
    io::{
        outboard::PreOrderOutboard,
        sync::{CreateOutboard, encode_ranges_validated},
    },
};
use bytes::Bytes;
use iroh::{
    Endpoint, EndpointAddr, EndpointId,
    endpoint::{Connection, ConnectionError, QuicTransportConfig, VarInt, WriteError, presets},
};
use iroh_blobs::protocol::{ChunkRangesSeq, Request};
use nrz_contract::artifact_ingress_v1::{self as wire, Open, Reply, Resume};
use std::{
    collections::BTreeMap,
    fs::File,
    future::Future,
    io::{self, Seek as _, SeekFrom, Write},
    net::SocketAddr,
    ops::{Bound, RangeBounds as _},
    pin::Pin,
    time::Duration,
};
use tokio::{io::AsyncReadExt as _, sync::mpsc, time::Instant};
use uuid::Uuid;

/// The caller freezes these files before admission. The selected receiver still
/// independently verifies each byte; a producer's filesystem is not authority.
pub struct FrozenBlob {
    pub hash: [u8; 32],
    pub bytes: u64,
    pub file: File,
}
pub struct TransferTarget {
    pub endpoint_id: String,
    pub direct_addresses: Vec<SocketAddr>,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub manifest_blake3: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PublicationOutcome {
    Published,
    /// The exact authenticated admit body must be reconciled; never start a new operation.
    Reconcile,
}
type Reconciliation = Box<
    dyn FnMut() -> Pin<Box<dyn Future<Output = anyhow::Result<PublicationOutcome>> + Send>> + Send,
>;

pub struct TransferProvider {
    endpoint: Endpoint,
    blobs: BTreeMap<[u8; 32], FrozenBlob>,
    #[cfg(test)]
    pub(crate) encoder_gate: std::sync::Mutex<Option<EncoderGate>>,
}
#[cfg(test)]
pub(crate) type EncoderGate = (
    tokio::sync::oneshot::Sender<()>,
    std::sync::mpsc::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
);
impl TransferProvider {
    /// Construct before API admission so its fresh peer ID is part of that input.
    pub async fn bind(blobs: impl IntoIterator<Item = FrozenBlob>) -> anyhow::Result<Self> {
        let mut catalog = BTreeMap::new();
        for blob in blobs {
            ensure!(
                blob.file.metadata()?.is_file() && blob.file.metadata()?.len() == blob.bytes,
                "frozen blob size changed"
            );
            if let Some(old) = catalog.insert(blob.hash, blob) {
                ensure!(
                    old.bytes == catalog[&old.hash].bytes,
                    "duplicate hash size mismatch"
                );
            }
        }
        let transport = QuicTransportConfig::builder()
            .max_concurrent_bidi_streams(VarInt::from_u32(4))
            .max_concurrent_uni_streams(VarInt::from_u32(0))
            .datagram_receive_buffer_size(None)
            .datagram_send_buffer_size(0)
            .build();
        let endpoint = Endpoint::builder(presets::Minimal)
            .clear_ip_transports()
            .bind_addr("0.0.0.0:0".parse::<SocketAddr>()?)?
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![wire::ALPN.to_vec()])
            .transport_config(transport)
            .bind()
            .await?;
        Ok(Self {
            endpoint,
            blobs: catalog,
            #[cfg(test)]
            encoder_gate: std::sync::Mutex::new(None),
        })
    }
    pub fn endpoint_id(&self) -> String {
        self.endpoint.id().to_string()
    }
    pub async fn publish(
        self,
        target: TransferTarget,
        grant: String,
        deadline: Instant,
    ) -> anyhow::Result<()> {
        self.publish_with_deadlines(target, grant, deadline, deadline)
            .await
    }
    /// Legacy receivers retain the original single-connection wire and behavior.
    pub async fn publish_with_deadlines(
        self,
        target: TransferTarget,
        grant: String,
        opening_deadline: Instant,
        deadline: Instant,
    ) -> anyhow::Result<()> {
        self.run(target, grant, opening_deadline, deadline, false, None)
            .await?;
        Ok(())
    }
    pub async fn publish_resume_with_deadlines(
        self,
        target: TransferTarget,
        grant: String,
        opening_deadline: Instant,
        deadline: Instant,
    ) -> anyhow::Result<PublicationOutcome> {
        self.run(target, grant, opening_deadline, deadline, true, None)
            .await
    }
    /// Reconcile ambiguous transport results without retiring the frozen producer.
    /// `Reconcile` from the callback means the original operation is still active.
    /// The callback retries transient observation failures within `deadline`; an
    /// error ends publication and retires the producer.
    pub async fn publish_resume_with_reconciliation<F, Fut>(
        self,
        target: TransferTarget,
        grant: String,
        opening_deadline: Instant,
        deadline: Instant,
        mut reconcile: F,
    ) -> anyhow::Result<PublicationOutcome>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<PublicationOutcome>> + Send + 'static,
    {
        self.run(
            target,
            grant,
            opening_deadline,
            deadline,
            true,
            Some(Box::new(move || Box::pin(reconcile()))),
        )
        .await
    }
    async fn run(
        self,
        target: TransferTarget,
        grant: String,
        opening_deadline: Instant,
        deadline: Instant,
        resume: bool,
        mut reconcile: Option<Reconciliation>,
    ) -> anyhow::Result<PublicationOutcome> {
        ensure!(
            opening_deadline <= deadline,
            "opening exceeds operation deadline"
        );
        // The detached owner drains its encoder even when its observer is cancelled.
        tokio::spawn(async move {
            let mut producer = FileProducer::default();
            #[cfg(test)]
            {
                producer.gate = self.encoder_gate.lock().unwrap().take();
            }
            let mut attempted = false;
            let result = tokio::time::timeout_at(
                deadline,
                self.publish_inner(
                    target,
                    grant,
                    opening_deadline,
                    deadline,
                    resume,
                    &mut attempted,
                    &mut producer,
                    &mut reconcile,
                ),
            )
            .await;
            let result = match result {
                Ok(result) => result,
                Err(_) if resume && attempted => Ok(PublicationOutcome::Reconcile),
                Err(error) => Err(anyhow::Error::new(error)
                    .context("Source V2 transfer deadline; original operation may remain pending")),
            };
            let retired = producer.retire().await;
            self.endpoint.close().await;
            if let Err(error) = retired
                && !consumer_closed(&error)
            {
                return Err(error);
            }
            result
        })
        .await?
    }
    #[allow(clippy::too_many_arguments)]
    async fn publish_inner(
        &self,
        target: TransferTarget,
        grant: String,
        opening_deadline: Instant,
        deadline: Instant,
        resume: bool,
        attempted: &mut bool,
        producer: &mut FileProducer,
        reconcile: &mut Option<Reconciliation>,
    ) -> anyhow::Result<PublicationOutcome> {
        ensure!(
            !target.operation_id.is_nil() && !target.session_id.is_nil(),
            "admitted operation identity required"
        );
        ensure!(
            !target.direct_addresses.is_empty() && target.direct_addresses.len() <= 8,
            "direct target addresses required"
        );
        let peer: EndpointId = target.endpoint_id.parse()?;
        let mut address = EndpointAddr::new(peer);
        for direct in &target.direct_addresses {
            ensure!(
                direct.port() != 0 && !direct.ip().is_unspecified() && !direct.ip().is_multicast(),
                "invalid admitted address"
            );
            address = address.with_ip_addr(*direct);
        }
        let open = Open {
            transfer_grant: grant,
        };
        ensure!(open.validate(), "bounded signed grant required");
        ensure!(
            Instant::now() < opening_deadline,
            "source admission expired before opening"
        );
        let mut resumable = resume;
        let mut attaching = false;
        let mut failures = 0u32;
        let can_reconcile = reconcile.is_some();
        loop {
            let until = if attaching {
                deadline
            } else {
                opening_deadline
            };
            let generation = async {
                let opening = tokio::time::timeout_at(
                    if resume {
                        until.min(Instant::now() + Duration::from_secs(5))
                    } else {
                        until
                    },
                    async {
                        let connection = match self
                            .endpoint
                            .connect(
                                address.clone(),
                                if resumable {
                                    wire::RESUME_ALPN
                                } else {
                                    wire::ALPN
                                },
                            )
                            .await
                        {
                            Ok(connection) => connection,
                            Err(error) if resumable && !attaching && unsupported_alpn(&error) => {
                                resumable = false;
                                self.endpoint.connect(address.clone(), wire::ALPN).await?
                            }
                            Err(error) => return Err(error.into()),
                        };
                        ensure!(
                            connection.remote_id() == peer,
                            "admission target peer changed"
                        );
                        let (mut send, mut receive) = connection.open_bi().await?;
                        *attempted = true;
                        if attaching {
                            write_frame(
                                &mut send,
                                &Resume {
                                    operation_id: target.operation_id,
                                    session_id: target.session_id,
                                },
                                wire::MAX_OPEN_BYTES,
                            )
                            .await?;
                        } else {
                            write_frame(&mut send, &open, wire::MAX_OPEN_BYTES).await?;
                        }
                        send.finish()?;
                        let first: Reply = read_frame(&mut receive, wire::MAX_REPLY_BYTES).await?;
                        ensure!(Instant::now() < until, "Source V2 opening deadline");
                        if (attaching || can_reconcile) && matches!(first, Reply::Unavailable) {
                            connection.close(0u32.into(), b"resume unavailable");
                            return Ok(None);
                        }
                        ensure!(
                            matches!(first, Reply::Accepted {operation_id,session_id}
                    if operation_id == target.operation_id && session_id == target.session_id),
                            "receiver did not accept exact granted session"
                        );
                        Ok::<_, anyhow::Error>(Some((connection, receive)))
                    },
                )
                .await?;
                let Some((connection, mut receive)) = opening? else {
                    return Ok(PublicationOutcome::Reconcile);
                };
                let result = self
                    .serve_generation(&connection, &mut receive, &target, producer, resumable)
                    .await;
                connection.close(0u32.into(), b"provider generation retired");
                result
            }
            .await;
            match generation {
                Ok(PublicationOutcome::Published) => return Ok(PublicationOutcome::Published),
                Ok(PublicationOutcome::Reconcile) => failures += 1,
                Err(error)
                    if resume
                        && (retryable_connection(&error)
                            || error.is::<tokio::time::error::Elapsed>()) =>
                {
                    failures += 1;
                    if failures < 4 && (!*attempted || resumable) {
                        attaching = *attempted;
                        tokio::time::sleep(Duration::from_millis(100u64 << (failures - 1))).await;
                        continue;
                    }
                    if !*attempted {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            }
            let Some(reconcile) = reconcile else {
                return Ok(PublicationOutcome::Reconcile);
            };
            loop {
                if reconcile().await? == PublicationOutcome::Published {
                    return Ok(PublicationOutcome::Published);
                }
                // ACTIVE is not terminal. Never renew the original transport budget.
                if resumable && failures < 4 {
                    attaching = true;
                    tokio::time::sleep(Duration::from_millis(100u64 << (failures - 1))).await;
                    break;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
    async fn serve_generation(
        &self,
        connection: &Connection,
        receive: &mut iroh::endpoint::RecvStream,
        target: &TransferTarget,
        producer: &mut FileProducer,
        resumable: bool,
    ) -> anyhow::Result<PublicationOutcome> {
        // Keep a partial framed control read alive while serially serving blobs.
        let terminal = read_frame::<Reply>(receive, wire::MAX_REPLY_BYTES);
        tokio::pin!(terminal);
        loop {
            tokio::select! {
                terminal = &mut terminal => {
                    ensure!(matches!(terminal?, Reply::Published {operation_id,session_id,manifest_blake3}
                        if operation_id == target.operation_id && session_id == target.session_id
                        && manifest_blake3 == target.manifest_blake3), "exact family publication not confirmed");
                    connection.close(0u32.into(), b"publication observed");
                    return Ok(PublicationOutcome::Published);
                }
                streams = connection.accept_bi() => {
                    let (mut writer, mut reader) = streams?;
                    let (request, _) = Request::read_async(&mut reader).await?;
                    let Request::Get(request) = request else { anyhow::bail!("only exact raw Get is supported"); };
                    let blob = self.blobs.get(request.hash.as_bytes())
                        .context("requested blob is outside frozen operation")?;
                    let ranges = if resumable { suffix_ranges(&request.ranges, blob.bytes)? } else {
                        ensure!(request.ranges == ChunkRangesSeq::from_ranges([ChunkRanges::all()]),
                            "only one whole raw root is supported");
                        ChunkRanges::all()
                    };
                    match serve_file(blob, ranges, &mut writer, producer).await {
                        Ok(()) => { writer.finish()?; }
                        Err(error) if resumable && stream_interrupted(&error) => {}
                        Err(error) => return Err(error),
                    }
                }
            }
        }
    }
}
#[derive(Default)]
struct FileProducer {
    task: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
    #[cfg(test)]
    gate: Option<EncoderGate>,
    #[cfg(test)]
    joined: Option<tokio::sync::oneshot::Sender<()>>,
}
impl FileProducer {
    async fn retire(&mut self) -> anyhow::Result<()> {
        let Some(task) = &mut self.task else {
            return Ok(());
        };
        let result = task.await;
        self.task = None;
        #[cfg(test)]
        if let Some(joined) = self.joined.take() {
            let _ = joined.send(());
        }
        result?
    }
}
async fn serve_file(
    blob: &FrozenBlob,
    ranges: ChunkRanges,
    writer: &mut iroh::endpoint::SendStream,
    producer: &mut FileProducer,
) -> anyhow::Result<()> {
    let mut file = blob.file.try_clone()?;
    let expected = blob.hash;
    let size = blob.bytes;
    let (tx, mut rx) = mpsc::channel::<Bytes>(1);
    ensure!(
        producer.task.is_none(),
        "previous file encoder has not retired"
    );
    #[cfg(test)]
    let gate = producer.gate.take().map(|(entered, release, joined)| {
        producer.joined = Some(joined);
        (entered, release)
    });
    producer.task = Some(tokio::task::spawn_blocking(
        move || -> anyhow::Result<()> {
            #[cfg(test)]
            if let Some((entered, release)) = gate {
                let _ = entered.send(());
                release.recv().context("injected file encoder release")?;
            }
            file.seek(SeekFrom::Start(0))?;
            ensure!(file.metadata()?.len() == size, "frozen blob changed");
            let mut outboard = PreOrderOutboard {
                root: [0; 32].into(),
                tree: BaoTree::new(size, iroh_blobs::store::IROH_BLOCK_SIZE),
                data: tempfile::tempfile()?,
            };
            outboard.init_from(&mut file)?;
            ensure!(
                outboard.root.as_bytes() == &expected,
                "frozen blob hash changed"
            );
            let mut output = ChunkWriter(tx);
            output.write_all(&size.to_le_bytes())?;
            encode_ranges_validated(&file, &outboard, &ranges, output)?;
            Ok(())
        },
    ));
    let mut error = None;
    while let Some(chunk) = rx.recv().await {
        if let Err(cause) = writer.write_all(&chunk).await {
            error = Some(cause);
            break;
        }
    }
    drop(rx);
    // Join the actual file reader even if QUIC cancellation closed its channel.
    let produced = producer.retire().await;
    if let Err(produced_error) = produced
        && (!consumer_closed(&produced_error) || error.is_none())
    {
        return Err(produced_error);
    }
    if let Some(error) = error {
        return Err(error.into());
    }
    Ok(())
}
fn suffix_ranges(sequence: &ChunkRangesSeq, bytes: u64) -> anyhow::Result<ChunkRanges> {
    let (root, ranges) = sequence
        .as_single()
        .context("only one raw root is supported")?;
    ensure!(root == 0, "only raw root index zero is supported");
    let mut iter = ranges.iter();
    let range = iter.next().context("empty raw root range")?;
    ensure!(
        iter.next().is_none() && matches!(range.end_bound(), Bound::Unbounded),
        "only one contiguous raw root suffix is supported"
    );
    let Bound::Included(start) = range.start_bound() else {
        anyhow::bail!("invalid suffix start");
    };
    ensure!(
        start.0 == 0 || start.0 < bytes.div_ceil(1024),
        "suffix starts past frozen root"
    );
    Ok(ranges.clone())
}
#[derive(Debug, thiserror::Error)]
#[error("provider observer closed")]
struct ConsumerClosed;
fn consumer_closed(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .and_then(|error| error.get_ref())
            .is_some_and(|error| error.is::<ConsumerClosed>())
    })
}
fn stream_interrupted(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(cause.downcast_ref::<WriteError>(),
        Some(WriteError::Stopped(code)) if *code == VarInt::from_u32(0))
    })
}
fn retryable_connection(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| matches!(cause.downcast_ref::<ConnectionError>(),
        Some(ConnectionError::Reset | ConnectionError::TimedOut))
        || matches!(cause.downcast_ref::<ConnectionError>(),
            Some(ConnectionError::ApplicationClosed(close)) if close.error_code == VarInt::from_u32(0)))
}
fn unsupported_alpn(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut cause = Some(error);
    while let Some(current) = cause {
        if matches!(current.downcast_ref::<ConnectionError>(),
            Some(ConnectionError::ConnectionClosed(close)) if u64::from(close.error_code) == 0x178)
            || matches!(current.downcast_ref::<ConnectionError>(),
                Some(ConnectionError::TransportError(error)) if u64::from(error.code) == 0x178)
        {
            return true;
        }
        cause = current.source();
    }
    false
}

struct ChunkWriter(mpsc::Sender<Bytes>);
impl Write for ChunkWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(16 * 1024) {
            self.0
                .blocking_send(Bytes::copy_from_slice(chunk))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, ConsumerClosed))?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub async fn write_frame<T: serde::Serialize>(
    writer: &mut iroh::endpoint::SendStream,
    value: &T,
    limit: usize,
) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(
        !bytes.is_empty() && bytes.len() <= limit,
        "oversized ingress frame"
    );
    writer
        .write_all(&(u32::try_from(bytes.len())?).to_be_bytes())
        .await?;
    writer.write_all(&bytes).await?;
    Ok(())
}
pub async fn read_frame<T: serde::de::DeserializeOwned>(
    reader: &mut iroh::endpoint::RecvStream,
    limit: usize,
) -> anyhow::Result<T> {
    let length = reader.read_u32().await? as usize;
    ensure!(length > 0 && length <= limit, "oversized ingress frame");
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
#[path = "ingress_v2_continuation_tests.rs"]
mod continuation_tests;

#[cfg(test)]
#[path = "ingress_v2_resume_tests.rs"]
mod resume_tests;
