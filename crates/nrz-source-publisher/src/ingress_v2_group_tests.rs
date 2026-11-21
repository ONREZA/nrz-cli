use super::resume_tests::{accept, pause_encoder, published, receiver, start_publish, target};
use super::*;
use bao_tree::{ChunkNum, io::BaoContentItem};
use iroh_blobs::{Hash, get::fsm};

async fn fixture(sizes: &[usize]) -> (TransferProvider, GetManyRequest, BTreeMap<Hash, Vec<u8>>) {
    let frozen = tempfile::tempdir().unwrap();
    let mut blobs = Vec::new();
    let mut contents = BTreeMap::new();
    for (index, size) in sizes.iter().copied().enumerate() {
        let mut bytes = vec![index as u8; size];
        for (to, from) in bytes.iter_mut().zip(index.to_le_bytes()) {
            *to = from;
        }
        let hash = Hash::from(*bao_tree::blake3::hash(&bytes).as_bytes());
        let name = index.to_string();
        std::fs::write(frozen.path().join(&name), &bytes).unwrap();
        blobs.push(FrozenTreeBlob {
            hash: *hash.as_bytes(),
            bytes: size as u64,
            path: name.into(),
        });
        contents.insert(hash, bytes);
    }
    let request = contents.keys().copied().collect();
    let provider = TransferProvider::bind_frozen_tree(frozen, blobs)
        .await
        .unwrap();
    (provider, request, contents)
}

pub(super) async fn fetch_group(
    connection: &Connection,
    request: GetManyRequest,
) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut child = fsm::start_get_many(connection.clone(), request.clone(), Default::default())
        .await?
        .map_err(|_| anyhow::anyhow!("nonempty group required"))?;
    let mut files = Vec::new();
    loop {
        ensure!(
            child.offset() == files.len() as u64,
            "unexpected group child offset"
        );
        let (mut content, size) = child.next(request.hashes[files.len()]).next().await?;
        let mut bytes = Vec::new();
        let end = loop {
            match content.next().await {
                fsm::BlobContentNext::More((next, item)) => {
                    if let BaoContentItem::Leaf(leaf) = item? {
                        ensure!(
                            leaf.offset == bytes.len() as u64,
                            "unexpected group leaf offset"
                        );
                        bytes.extend_from_slice(&leaf.data);
                    }
                    content = next;
                }
                fsm::BlobContentNext::Done(end) => break end,
            }
        };
        ensure!(bytes.len() as u64 == size, "incomplete group member");
        files.push(bytes);
        match end.next() {
            fsm::EndBlobNext::MoreChildren(next) => child = next,
            fsm::EndBlobNext::Closing(closing) => {
                closing.next().await?;
                break;
            }
        }
    }
    ensure!(
        files.len() == request.hashes.len(),
        "incomplete group response"
    );
    Ok(files)
}

#[tokio::test]
async fn get_many_transfers_whole_roots_on_one_stream_with_exact_eof() {
    for sizes in [
        vec![0, 1, 1024, MAX_GROUP_FILE_BYTES as usize],
        vec![1; MAX_GROUP_FILES],
        vec![MAX_GROUP_FILE_BYTES as usize; (MAX_GROUP_BYTES / MAX_GROUP_FILE_BYTES) as usize],
    ] {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (provider, request, contents) = fixture(&sizes).await;
            assert!(
                provider
                    .requested_members(Request::GetMany(request.clone()), true, true)
                    .is_ok()
            );
            let receiver = receiver(wire::GROUP_ALPN).await;
            let hash = request.hashes[0];
            let task = start_publish(provider, &receiver, hash);
            let (connection, mut control) = accept(&receiver, false).await;
            let files = fetch_group(&connection, request.clone()).await.unwrap();
            assert_eq!(
                files,
                request
                    .hashes
                    .iter()
                    .map(|hash| contents[hash].clone())
                    .collect::<Vec<_>>()
            );
            published(&mut control, hash).await;
            assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn get_many_prevalidates_every_hash_range_and_catalog_limit() {
    let (provider, request, _) = fixture(&[4, 8]).await;
    let mut invalid = vec![GetManyRequest::new(vec![], ChunkRangesSeq::empty())];
    let mut unknown = request.clone();
    unknown.hashes.push(Hash::from([255; 32]));
    invalid.push(unknown.hashes.into_iter().collect());
    let mut duplicate = request.clone();
    duplicate.hashes.push(duplicate.hashes[0]);
    invalid.push(duplicate);
    let mut reversed = request.clone();
    reversed.hashes.reverse();
    invalid.push(reversed);
    for ranges in [
        ChunkRangesSeq::all(),
        ChunkRangesSeq::root(),
        ChunkRangesSeq::from_ranges([ChunkRanges::empty(), ChunkRanges::all()]),
        ChunkRangesSeq::from_ranges([ChunkRanges::from(ChunkNum(1)..), ChunkRanges::all()]),
    ] {
        invalid.push(GetManyRequest::new(request.hashes.clone(), ranges));
    }
    for request in invalid {
        assert!(
            provider
                .requested_members(Request::GetMany(request), true, true)
                .is_err()
        );
    }
    for sizes in [
        vec![4; MAX_GROUP_FILES + 1],
        vec![MAX_GROUP_FILE_BYTES as usize + 1],
        vec![MAX_GROUP_FILE_BYTES as usize; (MAX_GROUP_BYTES / MAX_GROUP_FILE_BYTES + 1) as usize],
    ] {
        let (provider, request, _) = fixture(&sizes).await;
        assert!(
            provider
                .requested_members(Request::GetMany(request), true, true)
                .is_err()
        );
    }
}

#[tokio::test]
async fn unknown_later_member_is_rejected_before_any_response_byte() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (provider, mut request, _) = fixture(&[4]).await;
        request.hashes.push(Hash::from([255; 32]));
        request = request.hashes.into_iter().collect();
        let receiver = receiver(wire::GROUP_ALPN).await;
        let hash = request.hashes[0];
        let task = start_publish(provider, &receiver, hash);
        let (connection, _control) = accept(&receiver, false).await;
        let child = fsm::start_get_many(connection, request.clone(), Default::default())
            .await
            .unwrap()
            .unwrap();
        assert!(child.next(request.hashes[0]).next().await.is_err());
        assert!(
            task.await
                .unwrap()
                .unwrap_err()
                .to_string()
                .contains("outside frozen operation")
        );
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn interrupted_group_drains_encoder_before_next_request() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let (provider, request, contents) = fixture(&[16384; 32]).await;
        let path = provider._frozen.as_ref().unwrap().path().to_owned();
        let paused = pause_encoder(&provider);
        let receiver = receiver(wire::GROUP_ALPN).await;
        let hash = request.hashes[0];
        let task = start_publish(provider, &receiver, hash);
        let (connection, mut control) = accept(&receiver, false).await;
        let child = fsm::start_get_many(connection.clone(), request.clone(), Default::default())
            .await
            .unwrap()
            .unwrap();
        paused.entered.await.unwrap();
        drop(child);
        paused.release.send(()).unwrap();
        paused.joined.await.unwrap();
        assert!(path.exists());
        assert_eq!(
            fetch_group(&connection, request.clone()).await.unwrap(),
            request
                .hashes
                .iter()
                .map(|hash| contents[hash].clone())
                .collect::<Vec<_>>()
        );
        published(&mut control, hash).await;
        assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
        assert!(!path.exists());
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn group_observer_cancellation_retains_tree_until_encoder_join() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (provider, request, _) = fixture(&[16384; 2]).await;
        let path = provider._frozen.as_ref().unwrap().path().to_owned();
        let paused = pause_encoder(&provider);
        let receiver = receiver(wire::GROUP_ALPN).await;
        let hash = request.hashes[0];
        let deadline = Instant::now() + Duration::from_millis(500);
        let task = tokio::spawn(provider.publish_resume_with_deadlines(
            target(&receiver, hash),
            "original-grant".into(),
            deadline,
            deadline,
        ));
        let (connection, _control) = accept(&receiver, false).await;
        let child = fsm::start_get_many(connection.clone(), request, Default::default())
            .await
            .unwrap()
            .unwrap();
        paused.entered.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::sleep_until(deadline + Duration::from_millis(50)).await;
        assert!(path.exists());
        drop(child);
        paused.release.send(()).unwrap();
        paused.joined.await.unwrap();
        connection.closed().await;
        while path.exists() {
            tokio::task::yield_now().await;
        }
        receiver.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn protocol_selection_preserves_resume_despite_legacy_server_preference() {
    for (alpns, expected) in [
        (
            vec![
                wire::GROUP_ALPN.to_vec(),
                wire::RESUME_ALPN.to_vec(),
                wire::ALPN.to_vec(),
            ],
            wire::GROUP_ALPN,
        ),
        (
            vec![wire::ALPN.to_vec(), wire::RESUME_ALPN.to_vec()],
            wire::RESUME_ALPN,
        ),
    ] {
        tokio::time::timeout(Duration::from_secs(8), async {
            let receiver = Endpoint::builder(presets::Minimal)
                .clear_ip_transports()
                .bind_addr("127.0.0.1:0")
                .unwrap()
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(alpns)
                .bind()
                .await
                .unwrap();
            let (provider, request, contents) = fixture(&[1024, 4096]).await;
            let hash = request.hashes[0];
            let task = start_publish(provider, &receiver, hash);
            let (connection, _control) = accept(&receiver, false).await;
            assert_eq!(connection.alpn(), expected);
            let identity = connection.remote_id();
            connection.close(0u32.into(), b"injected negotiated protocol disconnect");
            let (connection, mut control) = accept(&receiver, true).await;
            assert_eq!(connection.alpn(), expected);
            assert_eq!(connection.remote_id(), identity);
            if expected == wire::GROUP_ALPN {
                assert_eq!(
                    fetch_group(&connection, request.clone()).await.unwrap(),
                    request
                        .hashes
                        .iter()
                        .map(|hash| contents[hash].clone())
                        .collect::<Vec<_>>()
                );
            } else {
                assert_eq!(
                    super::resume_tests::fetch(&connection, hash, 0, false).await,
                    contents[&hash]
                );
            }
            published(&mut control, hash).await;
            assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn stopping_after_a_verified_member_allows_a_new_group() {
    for disconnect in [false, true] {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (provider, request, _) = fixture(&[16384; 64]).await;
            let receiver = receiver(wire::GROUP_ALPN).await;
            let hash = request.hashes[0];
            let task = start_publish(provider, &receiver, hash);
            let (mut connection, mut control) = accept(&receiver, false).await;
            let child =
                fsm::start_get_many(connection.clone(), request.clone(), Default::default())
                    .await
                    .unwrap()
                    .unwrap();
            let (mut content, size) = child.next(hash).next().await.unwrap();
            assert_eq!(size, 16384);
            let end = loop {
                match content.next().await {
                    fsm::BlobContentNext::More((next, item)) => {
                        item.unwrap();
                        content = next;
                    }
                    fsm::BlobContentNext::Done(end) => break end,
                }
            };
            let fsm::EndBlobNext::MoreChildren(next) = end.next() else {
                panic!("remaining group members");
            };
            drop(next);
            if disconnect {
                let identity = connection.remote_id();
                connection.close(0u32.into(), b"injected interrupted group connection");
                (connection, control) = accept(&receiver, true).await;
                assert_eq!(connection.remote_id(), identity);
                assert_eq!(connection.alpn(), wire::GROUP_ALPN);
            }
            let remaining = request
                .hashes
                .into_iter()
                .skip(1)
                .collect::<GetManyRequest>();
            assert_eq!(
                fetch_group(&connection, remaining.clone())
                    .await
                    .unwrap()
                    .len(),
                remaining.hashes.len()
            );
            published(&mut control, hash).await;
            assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn later_member_mutation_terminates_group_without_publication() {
    for grow in [false, true] {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (provider, request, _) = fixture(&[16384; 4]).await;
            let blob = &provider.blobs[request.hashes.last().unwrap().as_bytes()];
            let BlobSource::Rooted(_, relative) = &blob.source else {
                panic!("rooted source");
            };
            let path = provider._frozen.as_ref().unwrap().path().join(relative);
            std::fs::write(path, vec![255; 16384 + usize::from(grow)]).unwrap();
            let receiver = receiver(wire::GROUP_ALPN).await;
            let hash = request.hashes[0];
            let task = start_publish(provider, &receiver, hash);
            let (connection, _control) = accept(&receiver, false).await;
            assert!(fetch_group(&connection, request).await.is_err());
            let error = task.await.unwrap().unwrap_err().to_string();
            assert!(
                error.contains(if grow {
                    "frozen blob changed"
                } else {
                    "frozen blob hash changed"
                }),
                "{error}"
            );
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn single_get_keeps_bao_block_boundaries_and_nonzero_suffixes() {
    for size in [0, 1, 16384, 16385] {
        tokio::time::timeout(Duration::from_secs(8), async {
            let (provider, request, contents) = fixture(&[size]).await;
            let receiver = receiver(wire::GROUP_ALPN).await;
            let hash = request.hashes[0];
            let task = start_publish(provider, &receiver, hash);
            let (connection, mut control) = accept(&receiver, false).await;
            assert_eq!(
                super::resume_tests::fetch(&connection, hash, 0, false).await,
                contents[&hash]
            );
            if size >= 16384 {
                assert_eq!(
                    super::resume_tests::fetch(&connection, hash, 1024, false).await,
                    contents[&hash][1024..]
                );
            }
            published(&mut control, hash).await;
            assert_eq!(task.await.unwrap().unwrap(), PublicationOutcome::Published);
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn small_member_size_and_content_changes_fail_before_the_header() {
    for grow in [false, true] {
        tokio::time::timeout(Duration::from_secs(5), async {
            let (provider, request, _) = fixture(&[64]).await;
            let hash = request.hashes[0];
            let path = provider._frozen.as_ref().unwrap().path().join("0");
            std::fs::write(path, vec![255; 64 + usize::from(grow)]).unwrap();
            let receiver = receiver(wire::GROUP_ALPN).await;
            let task = start_publish(provider, &receiver, hash);
            let (connection, _control) = accept(&receiver, false).await;
            let connected = fsm::start(
                connection,
                iroh_blobs::protocol::GetRequest::blob(hash),
                Default::default(),
            )
            .next()
            .await
            .unwrap();
            let fsm::ConnectedNext::StartRoot(root) = connected.next().await.unwrap() else {
                panic!("raw root");
            };
            assert!(root.next().next().await.is_err());
            let error = task.await.unwrap().unwrap_err().to_string();
            assert!(
                error.contains(if grow {
                    "frozen blob changed"
                } else {
                    "frozen blob hash changed"
                }),
                "{error}"
            );
            receiver.close().await;
        })
        .await
        .unwrap();
    }
}
