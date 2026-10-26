use super::*;
use iroh_blobs::protocol::Request;

#[tokio::test]
async fn a_valid_nonzero_suffix_is_not_reported_as_rejected() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut file = tempfile::tempfile()?;
        file.set_len(BYTES)?;
        let outboard = outboard(&mut file)?;
        let hash = Hash::from(*outboard.root.as_bytes());
        let start_offset = BYTES - 32 * 1024 * 1024;
        let ranges = ChunkRanges::from(ChunkNum(start_offset / 1024)..);
        let mut response = BYTES.to_le_bytes().to_vec();
        encode_ranges_validated(&file, &outboard, &ranges, &mut response)?;
        let provider = Endpoint::builder(presets::Minimal)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")?
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![wire::ALPN.to_vec()])
            .bind()
            .await?;
        let receiver = Endpoint::builder(presets::Minimal)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")?
            .relay_mode(iroh::RelayMode::Disabled)
            .bind()
            .await?;
        let serving = tokio::spawn({
            let provider = provider.clone();
            let ranges = ranges.clone();
            async move {
                let connection = provider
                    .accept()
                    .await
                    .context("incoming suffix request")?
                    .await?;
                let (mut send, mut receive) = connection.accept_bi().await?;
                let (request, _) = Request::read_async(&mut receive).await?;
                ensure!(request == Request::Get(GetRequest::blob_ranges(hash, ranges)));
                send.write_all(&response).await?;
                send.finish()?;
                send.stopped().await?;
                Ok::<_, anyhow::Error>(())
            }
        });
        let connection = receiver.connect(provider.addr(), wire::ALPN).await?;
        let received = fetch(&connection, hash, start_offset, None).await?;
        ensure!(received == BYTES - start_offset);
        serving.await??;
        receiver.close().await;
        provider.close().await;
        Ok::<_, anyhow::Error>(())
    })
    .await?
}
