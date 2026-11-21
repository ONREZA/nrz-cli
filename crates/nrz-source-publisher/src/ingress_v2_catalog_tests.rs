use super::resume_tests::{accept, fetch, pause_encoder, published, receiver, start_publish};
use super::*;
use iroh_blobs::Hash;

async fn frozen_blob_fixture() -> (TransferProvider, Hash, PathBuf) {
    let frozen = tempfile::tempdir().unwrap();
    let path = frozen.path().to_owned();
    let bytes = b"frozen";
    std::fs::write(path.join("blob"), bytes).unwrap();
    let hash = Hash::from(*bao_tree::blake3::hash(bytes).as_bytes());
    let provider = TransferProvider::bind_frozen_tree(
        frozen,
        [FrozenTreeBlob {
            hash: *hash.as_bytes(),
            bytes: bytes.len() as u64,
            path: "blob".into(),
        }],
    )
    .await
    .unwrap();
    (provider, hash, path)
}

async fn assert_get_header_rejected(connection: Connection, hash: Hash) {
    let connected = iroh_blobs::get::fsm::start(
        connection,
        iroh_blobs::protocol::GetRequest::blob(hash),
        Default::default(),
    )
    .next()
    .await
    .unwrap();
    let iroh_blobs::get::fsm::ConnectedNext::StartRoot(root) = connected.next().await.unwrap()
    else {
        panic!("raw root");
    };
    assert!(root.next().next().await.is_err());
}

#[cfg(unix)]
#[test]
fn catalog_and_transfer_fit_a_small_file_descriptor_limit() {
    const CHILD: &str = "NRZ_TEST_CATALOG_FD_LIMIT";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new("sh")
            .args(["-c", "ulimit -n 128 && exec \"$@\"", "catalog-fd-test"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ingress_v2::catalog_tests::catalog_and_transfer_fit_a_small_file_descriptor_limit",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        return;
    }
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(20), async {
                let frozen = tempfile::tempdir().unwrap();
                let path = frozen.path().to_owned();
                let mut blobs = Vec::new();
                let mut samples = Vec::new();
                for index in 0..2048_u32 {
                    let bytes = index.to_le_bytes();
                    let name = index.to_string();
                    std::fs::write(path.join(&name), bytes).unwrap();
                    let hash = Hash::from(*bao_tree::blake3::hash(&bytes).as_bytes());
                    blobs.push(FrozenTreeBlob {
                        hash: *hash.as_bytes(),
                        bytes: bytes.len() as u64,
                        path: name.into(),
                    });
                    if index % 1023 == 0 {
                        samples.push((hash, bytes));
                    }
                }
                // Duplicate digests retain one catalog entry and can still be requested repeatedly.
                std::fs::write(path.join("duplicate"), samples[0].1).unwrap();
                blobs.push(FrozenTreeBlob {
                    hash: *samples[0].0.as_bytes(),
                    bytes: 4,
                    path: "duplicate".into(),
                });
                let provider = TransferProvider::bind_frozen_tree(frozen, blobs)
                    .await
                    .unwrap();
                assert_eq!(provider.blobs.len(), 2048);
                let group = provider
                    .blobs
                    .keys()
                    .take(MAX_GROUP_FILES)
                    .map(|hash| Hash::from(*hash))
                    .collect::<GetManyRequest>();
                let receiver = receiver(wire::GROUP_ALPN).await;
                let task = start_publish(provider, &receiver, samples[0].0);
                let (connection, mut control) = accept(&receiver, false).await;
                for (hash, bytes) in samples.iter().chain(samples.first()) {
                    assert_eq!(fetch(&connection, *hash, 0, false).await, bytes);
                }
                let grouped = super::group_tests::fetch_group(&connection, group)
                    .await
                    .unwrap();
                assert_eq!(grouped.len(), MAX_GROUP_FILES);
                assert!(grouped.iter().all(|bytes| bytes.len() == 4));
                published(&mut control, samples[0].0).await;
                assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
                assert!(!path.exists());
                receiver.close().await;
            })
            .await
            .unwrap();
        });
}

#[tokio::test]
async fn duplicate_digest_size_mismatch_is_rejected() {
    let frozen = tempfile::tempdir().unwrap();
    let blobs = [1, 2].map(|bytes| FrozenTreeBlob {
        hash: [0; 32],
        bytes,
        path: "blob".into(),
    });
    assert!(
        TransferProvider::bind_frozen_tree(frozen, blobs)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("duplicate hash size mismatch")
    );
}

#[test]
fn exact_file_end_rejects_growth_and_truncation() {
    for bytes in [b"".as_slice(), b"a", b"ab", b"abc"] {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(bytes).unwrap();
        assert_eq!(ensure_file_end(&mut file, 2).is_ok(), bytes.len() == 2);
    }
}

#[tokio::test]
async fn frozen_replacement_and_size_mutation_are_terminal_on_demand() {
    for mutation in ["grow", "shrink", "symlink", "unlisted"] {
        tokio::time::timeout(Duration::from_secs(5), async {
            let receiver = receiver(wire::RESUME_ALPN).await;
            let (provider, hash, directory) = frozen_blob_fixture().await;
            let path = directory.join("blob");
            match mutation {
                "grow" => std::fs::write(&path, b"frozen extra").unwrap(),
                "shrink" => std::fs::write(&path, b"fro").unwrap(),
                "symlink" => {
                    std::fs::remove_file(&path).unwrap();
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("missing", &path).unwrap();
                }
                _ => {}
            }
            let task = start_publish(provider, &receiver, hash);
            let (connection, _control) = accept(&receiver, false).await;
            let request_hash = if mutation == "unlisted" {
                Hash::from([7; 32])
            } else {
                hash
            };
            assert_get_header_rejected(connection, request_hash).await;
            assert!(task.await.unwrap().is_err());
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn observer_cancellation_retains_tree_until_actual_encoder_retirement() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (provider, hash, path) = frozen_blob_fixture().await;
        let receiver = receiver(wire::RESUME_ALPN).await;
        let paused = pause_encoder(&provider);
        let deadline = Instant::now() + Duration::from_millis(500);
        let task = tokio::spawn(provider.publish_resume_with_deadlines(
            super::resume_tests::target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
        ));
        let (connection, _control) = accept(&receiver, false).await;
        let fetching = tokio::spawn({
            let connection = connection.clone();
            async move { assert_get_header_rejected(connection, hash).await }
        });
        paused.entered.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::sleep_until(deadline + Duration::from_millis(50)).await;
        assert!(
            path.exists(),
            "observer or deadline deleted an active encoder's tree"
        );
        paused.release.send(()).unwrap();
        paused.joined.await.unwrap();
        connection.closed().await;
        while path.exists() {
            tokio::task::yield_now().await;
        }
        fetching.await.unwrap();
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn original_fd_api_serves_an_unlinked_file() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let bytes = b"unlinked source owned by an existing FD caller";
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(bytes).unwrap();
        let hash = Hash::from(*bao_tree::blake3::hash(bytes).as_bytes());
        let provider = TransferProvider::bind([FrozenBlob {
            hash: *hash.as_bytes(),
            bytes: bytes.len() as u64,
            file,
        }])
        .await
        .unwrap();
        let receiver = receiver(wire::RESUME_ALPN).await;
        let task = start_publish(provider, &receiver, hash);
        let (connection, mut control) = accept(&receiver, false).await;
        assert_eq!(fetch(&connection, hash, 0, false).await, bytes);
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        receiver.close().await;
    })
    .await
    .unwrap();
}
