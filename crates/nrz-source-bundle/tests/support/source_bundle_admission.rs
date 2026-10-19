use super::*;
use futures::StreamExt;
use nrz_source_bundle::{
    SOURCE_BUNDLE_VERIFIER_FIXED_MEMORY_BYTES, SourceBundleVerificationFailure,
    SourceBundleVerificationMemory, source_bundle_verifier_max_decoder_memory_bytes,
};
use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
struct Account {
    current: AtomicU64,
    peak: AtomicU64,
    decoders: AtomicU64,
}

struct Credit {
    account: Arc<Account>,
    request: SourceBundleVerificationMemory,
}

impl Account {
    fn reserve(
        self: &Arc<Self>,
        request: SourceBundleVerificationMemory,
        limit: u64,
    ) -> Result<Credit, SourceBundleVerificationFailure> {
        let bytes = request_bytes(request);
        let previous = self
            .current
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(bytes).filter(|next| *next <= limit)
            })
            .map_err(|_| deferred())?;
        self.peak.fetch_max(previous + bytes, Ordering::SeqCst);
        if matches!(request, SourceBundleVerificationMemory::Decoder { .. }) {
            self.decoders.fetch_add(1, Ordering::SeqCst);
        }
        Ok(Credit {
            account: Arc::clone(self),
            request,
        })
    }
}

impl Drop for Credit {
    fn drop(&mut self) {
        self.account
            .current
            .fetch_sub(request_bytes(self.request), Ordering::SeqCst);
        if matches!(self.request, SourceBundleVerificationMemory::Decoder { .. }) {
            self.account.decoders.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn request_bytes(request: SourceBundleVerificationMemory) -> u64 {
    match request {
        SourceBundleVerificationMemory::Parser { bytes }
        | SourceBundleVerificationMemory::Decoder { bytes } => bytes,
    }
}

fn deferred() -> SourceBundleVerificationFailure {
    SourceBundleVerificationFailure {
        error_code: "SOURCE_VERIFICATION_MEMORY_DEFERRED".into(),
        message: "test memory pressure".into(),
        details: None,
    }
}

fn frame(body: &[u8], window_log: u32) -> Vec<u8> {
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), 9).unwrap();
    encoder.window_log(window_log).unwrap();
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

#[tokio::test]
async fn rejects_declared_manifest_memory_before_reading_its_body() {
    let mut header = tar::Header::new_ustar();
    header
        .set_path(SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
        .unwrap();
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(16 * 1024 * 1024);
    header.set_cksum();
    let compressed = Bytes::from(frame(header.as_bytes(), 12));
    let input = input_for(fixture_manifest(), &compressed);
    let mut requested = 0;
    let error = verify_source_bundle_stream_with_admission(
        input,
        stream::iter([Ok::<_, std::io::Error>(compressed)]),
        |request| {
            let result = match request {
                SourceBundleVerificationMemory::Parser { bytes } => {
                    requested = bytes;
                    Err(deferred())
                }
                SourceBundleVerificationMemory::Decoder { .. } => Ok(()),
            };
            std::future::ready(result)
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_VERIFICATION_MEMORY_DEFERRED");
    assert_eq!(requested, 64 * 1024 * 1024);
}

#[tokio::test]
async fn later_larger_frame_rechecks_credit_and_releases_all_guards_on_pressure() {
    let manifest = fixture_manifest();
    let tar = zstd::stream::decode_all(fixture_bundle(&manifest).as_ref()).unwrap();
    let manifest_end = 512
        + canonical_source_logical_manifest_json(&manifest)
            .len()
            .div_ceil(512)
            * 512;
    let first = frame(&tar[..manifest_end], 12);
    let second = frame(&tar[manifest_end..], 20);
    let mut joined = first.clone();
    joined.extend_from_slice(&second);
    let compressed = Bytes::from(joined);
    let input = input_for(manifest, &compressed);
    let valid = verify_source_bundle_stream_with_admission(
        input.clone(),
        stream::iter([Ok::<_, std::io::Error>(compressed)]),
        |_| std::future::ready(Ok(())),
    )
    .await
    .unwrap();
    assert_eq!(valid.summary.file_count, 2);

    let account = Arc::new(Account::default());
    let mut windows = Vec::new();
    let error = verify_source_bundle_stream_with_admission(
        input,
        stream::iter([
            Ok::<_, std::io::Error>(Bytes::from(first)),
            Ok(Bytes::from(second)),
        ]),
        |request| {
            if let SourceBundleVerificationMemory::Decoder { bytes } = request {
                windows.push(bytes);
                if windows.len() == 2 {
                    assert_eq!(account.decoders.load(Ordering::SeqCst), 0);
                    assert!(account.current.load(Ordering::SeqCst) > 0);
                    return std::future::ready(Err(deferred()));
                }
            }
            std::future::ready(account.reserve(request, u64::MAX))
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_VERIFICATION_MEMORY_DEFERRED");
    assert_eq!(windows.len(), 2);
    assert!(windows[1] > windows[0]);
    assert_eq!(account.current.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn admission_preserves_multiframe_skippable_and_trailing_error_checks() {
    let manifest = fixture_manifest();
    let compressed = fixture_bundle(&manifest);
    let tar = zstd::stream::decode_all(compressed.as_ref()).unwrap();
    let mut skippable = vec![0x50, 0x2a, 0x4d, 0x18, 3, 0, 0, 0];
    skippable.extend_from_slice(b"aux");
    let mut multiframe = frame(&tar[..700], 12);
    multiframe.extend_from_slice(&skippable);
    multiframe.extend_from_slice(&frame(&tar[700..], 20));
    for compressed in [
        compressed,
        Bytes::from(multiframe),
        Bytes::from_static(b"unsupported header"),
        Bytes::from_static(&[0x28, 0xb5, 0x2f, 0xfd, 0, 0x98]),
        Bytes::from_static(&[0x28, 0xb5, 0x2f, 0xfd, 0, 0]),
    ] {
        let input = input_for(manifest.clone(), &compressed);
        let expected = verify_source_bundle_bytes(input.clone(), compressed.clone()).await;
        let chunks = compressed
            .chunks(3)
            .map(|chunk| Ok::<_, std::io::Error>(Bytes::copy_from_slice(chunk)))
            .collect::<Vec<_>>();
        let actual =
            verify_source_bundle_stream_with_admission(input, stream::iter(chunks), |_| {
                std::future::ready(Ok(()))
            })
            .await;
        assert_eq!(
            actual.map_err(|error| error.error_code),
            expected.map_err(|error| error.error_code)
        );
    }
    let compressed = fixture_bundle(&manifest);
    let input = input_for(manifest, &compressed);
    let error = verify_source_bundle_stream_with_admission(
        input,
        stream::iter([Ok(compressed), Err("late source read failure")]),
        |_| std::future::ready(Ok(())),
    )
    .await
    .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_OBJECT_READ_FAILED");
}

#[tokio::test]
async fn eight_small_archives_share_the_existing_single_decoder_memory_bound() {
    let manifest = fixture_manifest();
    let tar = zstd::stream::decode_all(fixture_bundle(&manifest).as_ref()).unwrap();
    let compressed = Bytes::from(frame(&tar, 22));
    let input = input_for(manifest, &compressed);
    let account = Arc::new(Account::default());
    let limit = source_bundle_verifier_max_decoder_memory_bytes();
    let base = (0..8)
        .map(|_| {
            account
                .reserve(
                    SourceBundleVerificationMemory::Parser {
                        bytes: SOURCE_BUNDLE_VERIFIER_FIXED_MEMORY_BYTES,
                    },
                    limit,
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut started = Vec::new();
    let mut resume = Vec::new();
    let mut jobs = Vec::new();
    for _ in 0..8 {
        let (started_tx, started_rx) = futures::channel::oneshot::channel();
        let (resume_tx, resume_rx) = futures::channel::oneshot::channel();
        started.push(started_rx);
        resume.push(resume_tx);
        let input = input.clone();
        let account = Arc::clone(&account);
        let halfway = compressed.len() / 2;
        let first = compressed.slice(..halfway);
        let second = compressed.slice(halfway..);
        let chunks = stream::iter([Ok::<_, std::io::Error>(first)]).chain(
            stream::once(async move {
                started_tx.send(()).unwrap();
                resume_rx.await.unwrap();
                Ok(second)
            })
            .boxed(),
        );
        jobs.push(tokio::spawn(async move {
            verify_source_bundle_stream_with_admission(input, chunks, |request| {
                std::future::ready(account.reserve(request, limit))
            })
            .await
        }));
    }
    for receiver in started {
        receiver.await.unwrap();
    }
    assert_eq!(account.decoders.load(Ordering::SeqCst), 8);
    for sender in resume {
        sender.send(()).unwrap();
    }
    for job in jobs {
        assert_eq!(job.await.unwrap().unwrap().summary.file_count, 2);
    }
    assert!(account.peak.load(Ordering::SeqCst) < limit);
    drop(base);
    assert_eq!(account.current.load(Ordering::SeqCst), 0);
}
