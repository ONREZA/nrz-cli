use super::*;
use bao_tree::{ChunkNum, io::BaoContentItem};
use iroh_blobs::{Hash, get::fsm, protocol::GetRequest};

async fn receiver(alpn: &[u8]) -> Endpoint {
    Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![alpn.to_vec()])
        .bind()
        .await
        .unwrap()
}
fn target(receiver: &Endpoint, hash: Hash) -> TransferTarget {
    TransferTarget {
        endpoint_id: receiver.id().to_string(),
        direct_addresses: receiver.addr().ip_addrs().copied().collect(),
        operation_id: Uuid::from_u128(1),
        session_id: Uuid::from_u128(2),
        manifest_blake3: hash.to_string(),
    }
}
async fn fixture() -> (TransferProvider, Hash, Vec<u8>) {
    let bytes: Vec<u8> = (0..2 * 1024 * 1024).map(|n| (n % 251) as u8).collect();
    let hash = Hash::from(*bao_tree::blake3::hash(&bytes).as_bytes());
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(&bytes).unwrap();
    let provider = TransferProvider::bind([FrozenBlob {
        hash: *hash.as_bytes(),
        bytes: bytes.len() as u64,
        file,
    }])
    .await
    .unwrap();
    (provider, hash, bytes)
}
fn start_publish(
    provider: TransferProvider,
    receiver: &Endpoint,
    hash: Hash,
) -> tokio::task::JoinHandle<anyhow::Result<PublicationOutcome>> {
    let deadline = Instant::now() + Duration::from_secs(8);
    tokio::spawn(provider.publish_resume_with_deadlines(
        target(receiver, hash),
        "original-grant".into(),
        deadline,
        deadline,
    ))
}
async fn accept(receiver: &Endpoint, resumed: bool) -> (Connection, iroh::endpoint::SendStream) {
    let connection = receiver.accept().await.unwrap().await.unwrap();
    let (mut send, mut receive) = connection.accept_bi().await.unwrap();
    if resumed {
        let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
            .await
            .unwrap();
        assert_eq!(request.operation_id, Uuid::from_u128(1));
        assert_eq!(request.session_id, Uuid::from_u128(2));
    } else {
        let request: Open = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
            .await
            .unwrap();
        assert_eq!(request.transfer_grant, "original-grant");
    }
    write_frame(
        &mut send,
        &Reply::Accepted {
            operation_id: Uuid::from_u128(1),
            session_id: Uuid::from_u128(2),
        },
        wire::MAX_REPLY_BYTES,
    )
    .await
    .unwrap();
    (connection, send)
}
async fn published(send: &mut iroh::endpoint::SendStream, hash: Hash) {
    write_frame(
        send,
        &Reply::Published {
            operation_id: Uuid::from_u128(1),
            session_id: Uuid::from_u128(2),
            manifest_blake3: hash.to_string(),
        },
        wire::MAX_REPLY_BYTES,
    )
    .await
    .unwrap();
    send.finish().unwrap();
}
async fn fetch(connection: &Connection, hash: Hash, start: u64, interrupt: bool) -> Vec<u8> {
    let state = fsm::start(
        connection.clone(),
        GetRequest::blob_ranges(hash, ChunkRanges::from(ChunkNum(start / 1024)..)),
        Default::default(),
    )
    .next()
    .await
    .unwrap();
    let fsm::ConnectedNext::StartRoot(root) = state.next().await.unwrap() else {
        panic!("raw root");
    };
    let (mut content, _) = root.next().next().await.unwrap();
    let mut bytes = Vec::new();
    loop {
        match content.next().await {
            fsm::BlobContentNext::More((next, item)) => {
                if let BaoContentItem::Leaf(leaf) = item.unwrap() {
                    assert_eq!(leaf.offset, start + bytes.len() as u64);
                    bytes.extend_from_slice(&leaf.data);
                    if interrupt {
                        drop(next);
                        return bytes;
                    }
                }
                content = next;
            }
            fsm::BlobContentNext::Done(end) => {
                let fsm::EndBlobNext::Closing(closing) = end.next() else {
                    panic!("raw root end");
                };
                closing.next().await.unwrap();
                return bytes;
            }
        }
    }
}
#[test]
fn suffix_requests_are_bounded_to_one_nonempty_root() {
    for start in [0, 1, 31] {
        let ranges = ChunkRanges::from(ChunkNum(start)..);
        assert_eq!(
            suffix_ranges(&ChunkRangesSeq::from_ranges([ranges.clone()]), 32768).unwrap(),
            ranges
        );
    }
    for ranges in [
        ChunkRanges::empty(),
        ChunkRanges::from(ChunkNum(32)..),
        ChunkRanges::from(ChunkNum(0)..ChunkNum(4)),
    ] {
        assert!(suffix_ranges(&ChunkRangesSeq::from_ranges([ranges]), 32768).is_err());
    }
    assert!(suffix_ranges(&ChunkRangesSeq::all(), 32768).is_err());
    assert!(
        suffix_ranges(
            &ChunkRangesSeq::from_ranges([ChunkRanges::all(), ChunkRanges::all()]),
            32768
        )
        .is_err()
    );
    assert!(
        suffix_ranges(
            &ChunkRangesSeq::from_ranges([ChunkRanges::empty(), ChunkRanges::all()]),
            32768
        )
        .is_err()
    );
    assert!(suffix_ranges(&ChunkRangesSeq::root(), 0).is_ok());
}
#[tokio::test]
async fn interrupted_stream_resumes_a_verified_suffix_on_same_connection() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, bytes) = fixture().await;
        let task = start_publish(provider, &receiver, hash);
        let (connection, mut control) = accept(&receiver, false).await;
        let mut received = fetch(&connection, hash, 0, true).await;
        let suffix = fetch(&connection, hash, received.len() as u64, false).await;
        received.extend(suffix);
        assert_eq!(received, bytes);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        receiver.close().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn lost_connection_attaches_same_identity_and_resumes_verified_suffix() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, bytes) = fixture().await;
        let task = start_publish(provider, &receiver, hash);
        let (connection, _control) = accept(&receiver, false).await;
        let mut received = fetch(&connection, hash, 0, true).await;
        let identity = connection.remote_id();
        connection.close(0u32.into(), b"injected lost connection");
        let (connection, mut control) = accept(&receiver, true).await;
        assert_eq!(identity, connection.remote_id());
        received.extend(fetch(&connection, hash, received.len() as u64, false).await);
        assert_eq!(received, bytes);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        receiver.close().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn resume_unavailable_requires_authenticated_reconciliation() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, _) = fixture().await;
        let task = start_publish(provider, &receiver, hash);
        let (connection, _control) = accept(&receiver, false).await;
        connection.close(0u32.into(), b"published receipt lost");
        let connection = receiver.accept().await.unwrap().await.unwrap();
        let (mut send, mut receive) = connection.accept_bi().await.unwrap();
        let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
            .await
            .unwrap();
        assert_eq!(request.session_id, Uuid::from_u128(2));
        write_frame(&mut send, &Reply::Unavailable, wire::MAX_REPLY_BYTES)
            .await
            .unwrap();
        send.finish().unwrap();
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Reconcile);
        receiver.close().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn initial_unavailable_preserves_legacy_error_and_opt_in_reconciliation() {
    tokio::time::timeout(Duration::from_secs(6), async {
        for reconcile in [false, true] {
            let receiver = receiver(wire::RESUME_ALPN).await;
            let (provider, hash, _) = fixture().await;
            let task = if reconcile {
                let deadline = Instant::now() + Duration::from_secs(4);
                tokio::spawn(provider.publish_resume_with_reconciliation(
                    target(&receiver, hash),
                    "original-grant".into(),
                    deadline,
                    deadline,
                    || async { Ok(PublicationOutcome::Published) },
                ))
            } else {
                start_publish(provider, &receiver, hash)
            };
            let connection = receiver.accept().await.unwrap().await.unwrap();
            let (mut send, mut receive) = connection.accept_bi().await.unwrap();
            let request: Open = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                .await
                .unwrap();
            assert_eq!(request.transfer_grant, "original-grant");
            write_frame(&mut send, &Reply::Unavailable, wire::MAX_REPLY_BYTES)
                .await
                .unwrap();
            send.finish().unwrap();
            let result = task.await.unwrap();
            if reconcile {
                assert_eq!(result.unwrap(), PublicationOutcome::Published);
            } else {
                assert_eq!(
                    result.unwrap_err().to_string(),
                    "receiver did not accept exact granted session"
                );
            }
            receiver.close().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unsupported_resume_alpn_falls_back_to_legacy_open_only() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let receiver = receiver(wire::ALPN).await;
        let (provider, hash, _) = fixture().await;
        let task = start_publish(provider, &receiver, hash);
        assert!(receiver.accept().await.unwrap().await.is_err());
        let (connection, mut control) = accept(&receiver, false).await;
        assert_eq!(connection.alpn(), wire::ALPN);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        receiver.close().await;
    })
    .await
    .unwrap();
}
#[test]
fn application_stream_codes_are_not_blanket_retryable() {
    assert!(stream_interrupted(&WriteError::Stopped(0u32.into()).into()));
    assert!(!stream_interrupted(
        &WriteError::Stopped(42u32.into()).into()
    ));
    assert!(!retryable_connection(
        &iroh::endpoint::ReadError::Reset(0u32.into()).into()
    ));
    assert!(!retryable_connection(
        &serde_json::from_str::<Open>("{}").err().unwrap().into()
    ));
}

#[tokio::test]
async fn frozen_integrity_failure_and_malformed_request_are_terminal() {
    for corrupt in [false, true] {
        tokio::time::timeout(Duration::from_secs(5), async {
            let receiver = receiver(wire::RESUME_ALPN).await;
            let (provider, hash, _) = fixture().await;
            if corrupt {
                let mut file = provider.blobs[hash.as_bytes()].file.try_clone().unwrap();
                file.seek(SeekFrom::Start(0)).unwrap();
                file.write_all(&[255]).unwrap();
            }
            let deadline = Instant::now() + Duration::from_secs(4);
            let task = tokio::spawn(provider.publish_resume_with_deadlines(
                target(&receiver, hash),
                "original-grant".into(),
                deadline,
                deadline,
            ));
            let (connection, _control) = accept(&receiver, false).await;
            if corrupt {
                let state = fsm::start(
                    connection.clone(),
                    GetRequest::blob(hash),
                    Default::default(),
                )
                .next()
                .await
                .unwrap();
                let fsm::ConnectedNext::StartRoot(root) = state.next().await.unwrap() else {
                    panic!("raw root");
                };
                assert!(root.next().next().await.is_err());
            } else {
                let (mut send, _receive) = connection.open_bi().await.unwrap();
                send.write_all(&[255]).await.unwrap();
                send.finish().unwrap();
            }
            let error = task.await.unwrap().unwrap_err();
            if corrupt {
                assert!(error.to_string().contains("frozen blob hash changed"));
            } else {
                assert!(
                    error
                        .to_string()
                        .contains("failed to deserialize request type")
                );
            }
            assert!(
                tokio::time::timeout(Duration::from_millis(50), receiver.accept())
                    .await
                    .is_err()
            );
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn a_lost_accepted_reply_uses_resume_and_connection_failures_are_bounded() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, _) = fixture().await;
        let deadline = Instant::now() + Duration::from_secs(4);
        let task = tokio::spawn(provider.publish_resume_with_deadlines(
            target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
        ));
        let mut identity = None;
        for generation in 0..4 {
            let connection = receiver.accept().await.unwrap().await.unwrap();
            let (_, mut receive) = connection.accept_bi().await.unwrap();
            if generation == 0 {
                let _: Open = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                    .await
                    .unwrap();
                identity = Some(connection.remote_id());
            } else {
                let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                    .await
                    .unwrap();
                assert_eq!(request.operation_id, Uuid::from_u128(1));
                assert_eq!(request.session_id, Uuid::from_u128(2));
                assert_eq!(identity, Some(connection.remote_id()));
            }
            connection.close(0u32.into(), b"injected lost acceptance");
        }
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Reconcile);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), receiver.accept())
                .await
                .is_err()
        );
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn active_encoder_is_actually_joined_before_reconnect_generation() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, bytes) = fixture().await;
        let (entered, entered_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let (joined, mut joined_rx) = tokio::sync::oneshot::channel();
        *provider.encoder_gate.lock().unwrap() = Some((entered, release_rx, joined));
        let deadline = Instant::now() + Duration::from_secs(6);
        let task = tokio::spawn(provider.publish_resume_with_deadlines(
            target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
        ));
        let (connection, _control) = accept(&receiver, false).await;
        let request = tokio::spawn({
            let connection = connection.clone();
            async move {
                let connected = fsm::start(connection, GetRequest::blob(hash), Default::default())
                    .next()
                    .await
                    .unwrap();
                let fsm::ConnectedNext::StartRoot(root) = connected.next().await.unwrap() else {
                    panic!("raw root");
                };
                assert!(root.next().next().await.is_err());
            }
        });
        entered_rx.await.unwrap();
        // The blocking file worker is alive when QUIC dies, without a prior STOP_SENDING.
        connection.close(0u32.into(), b"injected active encoder disconnect");
        assert!(
            tokio::time::timeout(Duration::from_millis(100), receiver.accept())
                .await
                .is_err()
        );
        assert!(matches!(
            joined_rx.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        release.send(()).unwrap();
        let (connection, mut control) = accept(&receiver, true).await;
        assert!(
            joined_rx.try_recv().is_ok(),
            "new generation started before actual encoder join"
        );
        assert_eq!(fetch(&connection, hash, 0, false).await, bytes);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        request.await.unwrap();
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn active_reconciliation_keeps_frozen_producer_until_publication() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let frozen = tempfile::tempdir().unwrap();
        let path = frozen.path().join("original");
        let bytes = b"exact frozen input across authenticated reconciliation";
        std::fs::write(&path, bytes).unwrap();
        let hash = Hash::from(*bao_tree::blake3::hash(bytes).as_bytes());
        let provider = TransferProvider::bind([FrozenBlob {
            hash: *hash.as_bytes(),
            bytes: bytes.len() as u64,
            file: File::open(&path).unwrap(),
        }])
        .await
        .unwrap();
        let identity = provider.endpoint_id();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reconciliations = calls.clone();
        let deadline = Instant::now() + Duration::from_secs(6);
        let task = tokio::spawn(provider.publish_resume_with_reconciliation(
            target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
            move || {
                assert!(frozen.path().join("original").exists());
                assert!(Instant::now() < deadline);
                let active = reconciliations.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;
                async move {
                    Ok(if active {
                        PublicationOutcome::Reconcile
                    } else {
                        PublicationOutcome::Published
                    })
                }
            },
        ));
        let (connection, _control) = accept(&receiver, false).await;
        assert_eq!(connection.remote_id().to_string(), identity);
        connection.close(0u32.into(), b"lost first acceptance");
        for active in [true, false] {
            let connection = receiver.accept().await.unwrap().await.unwrap();
            assert_eq!(connection.remote_id().to_string(), identity);
            let (mut send, mut receive) = connection.accept_bi().await.unwrap();
            let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                .await
                .unwrap();
            assert_eq!(request.operation_id, Uuid::from_u128(1));
            assert_eq!(request.session_id, Uuid::from_u128(2));
            write_frame(&mut send, &Reply::Unavailable, wire::MAX_REPLY_BYTES)
                .await
                .unwrap();
            send.finish().unwrap();
            connection.closed().await;
            if active {
                let (connection, _control) = accept(&receiver, true).await;
                assert_eq!(connection.remote_id().to_string(), identity);
                assert_eq!(fetch(&connection, hash, 0, false).await, bytes);
                assert!(path.exists(), "ACTIVE retired the frozen tree");
                connection.close(0u32.into(), b"publication receipt lost");
            }
        }
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert!(!path.exists(), "terminal publication retained frozen input");
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn active_reconciliation_never_renews_transport_budget_or_operation_deadline() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, _) = fixture().await;
        let frozen = tempfile::tempdir().unwrap();
        let path = frozen.path().to_owned();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reconciliations = calls.clone();
        let deadline = Instant::now() + Duration::from_secs(3);
        let task = tokio::spawn(provider.publish_resume_with_reconciliation(
            target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
            move || {
                assert!(frozen.path().exists());
                reconciliations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(PublicationOutcome::Reconcile) }
            },
        ));
        for generation in 0..4 {
            let connection = receiver.accept().await.unwrap().await.unwrap();
            let (_, mut receive) = connection.accept_bi().await.unwrap();
            if generation == 0 {
                let _: Open = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                    .await
                    .unwrap();
            } else {
                let request: Resume = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                    .await
                    .unwrap();
                assert_eq!(request.operation_id, Uuid::from_u128(1));
                assert_eq!(request.session_id, Uuid::from_u128(2));
            }
            connection.close(0u32.into(), b"exhaust original transport budget");
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(200), receiver.accept())
                .await
                .is_err()
        );
        assert!(path.exists());
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Reconcile);
        assert!(Instant::now() >= deadline);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(!path.exists());
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn active_reconciliation_reattaches_after_four_connection_failures() {
    tokio::time::timeout(Duration::from_secs(14), async {
        let receiver = receiver(wire::RESUME_ALPN).await;
        let (provider, hash, bytes) = fixture().await;
        let identity = provider.endpoint_id();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reconciliations = calls.clone();
        let opening = Instant::now() + Duration::from_secs(2);
        let deadline = Instant::now() + Duration::from_secs(11);
        let task = tokio::spawn(provider.publish_resume_with_reconciliation(
            target(&receiver, hash),
            "original-grant".into(),
            opening,
            deadline,
            move || {
                reconciliations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(PublicationOutcome::Reconcile) }
            },
        ));
        for generation in 0..4 {
            let (connection, _control) = accept(&receiver, generation > 0).await;
            assert_eq!(connection.remote_id().to_string(), identity);
            connection.close(0u32.into(), b"injected retryable connection failure");
        }
        let exhausted_at = Instant::now();
        let (connection, mut control) =
            tokio::time::timeout(Duration::from_secs(7), accept(&receiver, true))
                .await
                .expect("ACTIVE must permit useful continuation after four transport failures");
        assert!(Instant::now() - exhausted_at >= Duration::from_secs(5));
        assert!(
            Instant::now() > opening,
            "resume must not reopen the expired grant"
        );
        assert!(Instant::now() < deadline);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(connection.remote_id().to_string(), identity);
        assert_eq!(fetch(&connection, hash, 0, false).await, bytes);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        receiver.close().await;
    })
    .await
    .unwrap();
}
