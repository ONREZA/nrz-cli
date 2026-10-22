use super::*;

#[tokio::test]
async fn retry_budget_bounds_a_request_that_never_completes() {
    let result = retry_control_plane(
        &NoopPublicationObserver,
        "pending",
        Duration::from_millis(5),
        std::future::pending::<Result<(), SourcePublicationError>>,
    )
    .await;
    assert!(matches!(result, Err(SourcePublicationError::Deadline(_))));
}

#[test]
fn retry_policy_preserves_http_status_when_an_error_body_is_not_json() {
    let error = StructuredControlPlaneError {
        status: 503,
        code: "HTTP_503".into(),
        message: "Service Unavailable".into(),
        retry_after: Some(Duration::from_secs(3)),
        details: None,
    }
    .into();
    assert_eq!(retry_hint(&error), Some(Some(Duration::from_secs(3))));
}

#[tokio::test]
async fn source_v2_deadline_joins_the_actual_file_encoder_before_returning() {
    source_v2_blocked_encoder_retirement(false).await;
}

#[tokio::test]
async fn source_v2_cancelled_observer_retains_the_actual_file_encoder_owner() {
    source_v2_blocked_encoder_retirement(true).await;
}

async fn source_v2_blocked_encoder_retirement(cancel_observer: bool) {
    use crate::ingress_v2::{
        FrozenBlob, TransferProvider, TransferTarget, read_frame, write_frame,
    };
    use iroh::{Endpoint, RelayMode, endpoint::presets};
    use iroh_blobs::{
        Hash,
        get::fsm,
        protocol::{ChunkRangesSeq, GetRequest},
    };
    use nrz_contract::artifact_ingress_v1::{self as ingress, Open, Reply};
    use std::{io::Write as _, os::unix::fs::MetadataExt as _};
    use tokio::time::timeout;

    struct ReleaseOnDrop(std::sync::mpsc::Sender<()>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    fn open_handles(identity: (u64, u64)) -> usize {
        std::fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| std::fs::metadata(entry.path()).ok())
            .filter(|stat| (stat.dev(), stat.ino()) == identity)
            .count()
    }

    let watchdog = Duration::from_secs(10);
    let receiver = Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap())
        .unwrap()
        .relay_mode(RelayMode::Disabled)
        .alpns(vec![ingress::ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    let bytes = b"original frozen provider inode";
    let hash = Hash::new(bytes);
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(bytes).unwrap();
    let stat = file.metadata().unwrap();
    let identity = (stat.dev(), stat.ino());
    let provider = TransferProvider::bind([FrozenBlob {
        hash: *hash.as_bytes(),
        bytes: bytes.len() as u64,
        file,
    }])
    .await
    .unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release = ReleaseOnDrop(release_tx);
    *provider.encoder_gate.lock().unwrap() = Some((entered_tx, release_rx));
    let operation_id = Uuid::now_v7();
    let session_id = Uuid::now_v7();
    let target = TransferTarget {
        endpoint_id: receiver.id().to_string(),
        direct_addresses: receiver.bound_sockets(),
        operation_id,
        session_id,
        manifest_blake3: hash.to_string(),
    };
    let mut observer = tokio::spawn(provider.publish(
        target,
        "bounded test grant".into(),
        Instant::now() + Duration::from_secs(30),
    ));
    let connection = timeout(watchdog, async {
        receiver.accept().await.unwrap().await.unwrap()
    })
    .await
    .unwrap();
    let (mut send, mut receive) = timeout(watchdog, connection.accept_bi())
        .await
        .unwrap()
        .unwrap();
    let _: Open = read_frame(&mut receive, ingress::MAX_OPEN_BYTES)
        .await
        .unwrap();
    write_frame(
        &mut send,
        &Reply::Accepted {
            operation_id,
            session_id,
        },
        ingress::MAX_REPLY_BYTES,
    )
    .await
    .unwrap();
    let request = tokio::spawn(async move {
        let connected = fsm::start(
            connection,
            GetRequest::new(
                hash,
                ChunkRangesSeq::from_ranges([bao_tree::ChunkRanges::all()]),
            ),
            Default::default(),
        )
        .next()
        .await?;
        let fsm::ConnectedNext::StartRoot(root) = connected.next().await? else {
            anyhow::bail!("raw root required");
        };
        let _ = root.next().next().await;
        Ok::<_, anyhow::Error>(())
    });
    timeout(watchdog, entered_rx).await.unwrap().unwrap();
    assert!(
        open_handles(identity) >= 2,
        "original and encoder handles must remain owned"
    );
    if cancel_observer {
        observer.abort();
        assert!((&mut observer).await.unwrap_err().is_cancelled());
    }
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    // A live blocking encoder inhibits Tokio's paused-clock auto-advance.
    // Trigger the deadline explicitly, then observe retirement in real time.
    tokio::time::resume();
    if !cancel_observer {
        assert!(
            timeout(Duration::from_millis(10), &mut observer)
                .await
                .is_err(),
            "deadline returned before the blocked file encoder actually retired"
        );
    }
    assert!(
        open_handles(identity) >= 2,
        "cancellation released file custody early"
    );
    drop(release);
    if !cancel_observer {
        assert!(timeout(watchdog, observer).await.unwrap().unwrap().is_err());
    }
    // The detached owner also closes its original inode only after actual join.
    timeout(watchdog, async {
        while open_handles(identity) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("actual provider handles did not retire");
    timeout(watchdog, request).await.unwrap().unwrap().unwrap();
    receiver.close().await;
}
