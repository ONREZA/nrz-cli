use std::io;
use std::mem::MaybeUninit;

use tokio::io::{AsyncRead, AsyncReadExt};
use zstd::zstd_safe::{get_error_name, zstd_sys};

use super::{SourceBundleVerificationFailure, source_decode_failure};

// Preserve libzstd's default streaming limit rather than impose a producer-specific window.
const MAX_WINDOW_BYTES: u64 = 1 << zstd_sys::ZSTD_WINDOWLOG_LIMIT_DEFAULT;
const MAX_HEADER_BYTES: usize = zstd_sys::ZSTD_FRAMEHEADERSIZE_MAX as usize;

pub fn source_bundle_verifier_max_decoder_memory_bytes() -> u64 {
    // SAFETY: This pure estimator accepts any window size supported by libzstd.
    unsafe { zstd_sys::ZSTD_estimateDStreamSize(MAX_WINDOW_BYTES as usize) as u64 }
}

pub(super) async fn read_frame_header<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<(Vec<u8>, u64)>, SourceBundleVerificationFailure> {
    let mut header = vec![0; 1];
    if reader
        .read(&mut header)
        .await
        .map_err(source_decode_failure)?
        == 0
    {
        return Ok(None);
    }
    loop {
        let mut frame = MaybeUninit::<zstd_sys::ZSTD_FrameHeader>::uninit();
        // SAFETY: libzstd reads the supplied slice length and initializes frame only on success.
        let needed = unsafe {
            zstd_sys::ZSTD_getFrameHeader(frame.as_mut_ptr(), header.as_ptr().cast(), header.len())
        };
        check_zstd(needed)?;
        if needed == 0 {
            // SAFETY: A zero return from ZSTD_getFrameHeader guarantees initialization.
            let frame = unsafe { frame.assume_init() };
            if frame.windowSize > MAX_WINDOW_BYTES {
                return Err(source_decode_failure(io::Error::other(
                    "Frame requires too much memory for decoding",
                )));
            }
            // SAFETY: The checked window fits libzstd's supported streaming range.
            let bytes = unsafe { zstd_sys::ZSTD_estimateDStreamSize(frame.windowSize as usize) };
            check_zstd(bytes)?;
            return Ok(Some((header, bytes as u64)));
        }
        if needed <= header.len() || needed > MAX_HEADER_BYTES {
            return Err(source_decode_failure(io::Error::other(
                "Invalid zstd frame header length",
            )));
        }
        let start = header.len();
        header.resize(needed, 0);
        reader
            .read_exact(&mut header[start..])
            .await
            .map_err(source_decode_failure)?;
    }
}

fn check_zstd(code: usize) -> Result<(), SourceBundleVerificationFailure> {
    // SAFETY: ZSTD_isError accepts any size_t, including successful sizes.
    if unsafe { zstd_sys::ZSTD_isError(code) } != 0 {
        return Err(source_decode_failure(io::Error::other(get_error_name(
            code,
        ))));
    }
    Ok(())
}
