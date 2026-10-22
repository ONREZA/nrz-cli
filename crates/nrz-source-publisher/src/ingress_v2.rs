//! Ephemeral provider for one frozen Source V2 operation on its original connection.
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
    endpoint::{QuicTransportConfig, VarInt, presets},
};
use iroh_blobs::protocol::{ChunkRangesSeq, Request};
use nrz_contract::artifact_ingress_v1::{self as wire, Open, Reply};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Seek as _, SeekFrom, Write},
    net::SocketAddr,
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
        // The actual worker owns files and endpoint independently of its caller.
        // Dropping the observer detaches it; its deadline still drains and joins
        // any blocking file encoder before those owner handles can retire.
        tokio::spawn(async move {
            let mut producer = FileProducer::default();
            #[cfg(test)]
            {
                producer.gate = self.encoder_gate.lock().unwrap().take();
            }
            let result =
                tokio::time::timeout_at(deadline, self.publish_inner(target, grant, &mut producer))
                    .await
                    .context("Source V2 transfer deadline; original operation may remain pending")
                    .and_then(|result| result);
            // timeout cancels the QUIC writer/receiver and closes its channel,
            // but the JoinHandle lives outside that cancellable future.
            let retired = producer.retire().await;
            self.endpoint.close().await;
            result.and(retired)
        })
        .await?
    }
    async fn publish_inner(
        &self,
        target: TransferTarget,
        grant: String,
        producer: &mut FileProducer,
    ) -> anyhow::Result<()> {
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
        for direct in target.direct_addresses {
            ensure!(
                direct.port() != 0 && !direct.ip().is_unspecified() && !direct.ip().is_multicast(),
                "invalid admitted address"
            );
            address = address.with_ip_addr(direct);
        }
        let connection = self.endpoint.connect(address, wire::ALPN).await?;
        ensure!(
            connection.remote_id() == peer,
            "admission target peer changed"
        );
        let (mut send, mut receive) = connection.open_bi().await?;
        let open = Open {
            transfer_grant: grant,
        };
        ensure!(open.validate(), "bounded signed grant required");
        write_frame(&mut send, &open, wire::MAX_OPEN_BYTES).await?;
        send.finish()?;
        let first: Reply = read_frame(&mut receive, wire::MAX_REPLY_BYTES).await?;
        ensure!(
            matches!(first,Reply::Accepted {operation_id,session_id} if operation_id==target.operation_id && session_id==target.session_id),
            "receiver did not accept exact granted session"
        );
        // Keep the framed read alive across blob requests; cancelling a partial
        // frame read would consume its prefix and desynchronize the next read.
        let terminal = read_frame::<Reply>(&mut receive, wire::MAX_REPLY_BYTES);
        tokio::pin!(terminal);
        loop {
            tokio::select! {
                terminal=&mut terminal=> {
                    ensure!(matches!(terminal?,Reply::Published {operation_id,session_id,manifest_blake3} if operation_id==target.operation_id && session_id==target.session_id && manifest_blake3==target.manifest_blake3),"exact family publication not confirmed");
                    connection.close(0u32.into(),b"publication observed");return Ok(());
                }
                streams=connection.accept_bi()=> {
                    let (mut writer,mut reader)=streams?;
                    let (request,_)=Request::read_async(&mut reader).await?;
                    let Request::Get(request)=request else {anyhow::bail!("only exact raw Get is supported");};
                    ensure!(request.ranges==ChunkRangesSeq::from_ranges([ChunkRanges::all()]),"only one whole raw root is supported");
                    let blob=self.blobs.get(request.hash.as_bytes()).context("requested blob is outside frozen operation")?;
                    serve_file(blob,&mut writer,producer).await?;writer.finish()?;
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
}
impl FileProducer {
    async fn retire(&mut self) -> anyhow::Result<()> {
        let Some(task) = &mut self.task else {
            return Ok(());
        };
        let result = task.await;
        self.task = None;
        result?
    }
}
async fn serve_file(
    blob: &FrozenBlob,
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
    let gate = producer.gate.take();
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
            encode_ranges_validated(&file, &outboard, &ChunkRanges::all(), output)?;
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
    if let Some(error) = error {
        return Err(error.into());
    }
    produced
}
struct ChunkWriter(mpsc::Sender<Bytes>);
impl Write for ChunkWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(16 * 1024) {
            self.0
                .blocking_send(Bytes::copy_from_slice(chunk))
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::BrokenPipe, "provider observer closed")
                })?;
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
