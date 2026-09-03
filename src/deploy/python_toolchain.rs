use std::ffi::{OsStr, OsString};
#[cfg(windows)]
use std::io::Cursor;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::detect::python::{
    PYTHON_INTERPRETER, PYTHON_RUNTIME_VERSION, PYTHON_SITE_PACKAGES_ROOT,
};

const UV_VERSION: &str = "0.10.0";
const UV_RELEASE_ORIGIN: &str = "https://github.com";
const MAX_UV_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UV_BINARY_BYTES: u64 = 96 * 1024 * 1024;
const PLATFORM_PYTHON_TARGET: &str = "x86_64-manylinux_2_39";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PythonInstallMode {
    ManagedLocal,
    PinnedPlatform,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct PythonInstallCommand {
    pub(super) program: PathBuf,
    pub(super) arguments: Vec<OsString>,
    pub(super) display: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ArchiveFormat {
    TarGz,
    Zip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct UvArtifact {
    pub(super) target: &'static str,
    archive_name: &'static str,
    pub(super) archive_sha256: &'static str,
    pub(super) binary_sha256: &'static str,
    binary_name: &'static str,
    pub(super) format: ArchiveFormat,
}

pub(super) async fn resolve() -> anyhow::Result<PathBuf> {
    let artifact = artifact_for(std::env::consts::OS, std::env::consts::ARCH)?;
    let cached_path = default_cache_root()?
        .join("nrz")
        .join("python-tools")
        .join("uv")
        .join(UV_VERSION)
        .join(artifact.target)
        .join(artifact.binary_name);

    if cached_binary_is_valid(&cached_path, artifact).await? {
        return Ok(cached_path);
    }

    let archive = download_archive(artifact).await?;
    let binary = extract_binary(&archive, artifact)?;
    if sha256_hex(&binary) != artifact.binary_sha256 {
        bail!("extracted uv binary does not match the CLI pin");
    }
    install_binary(&cached_path, &binary, artifact).await?;
    Ok(cached_path)
}

pub(super) fn install_command(
    manifest: &str,
    mode: PythonInstallMode,
    host_os: &str,
    host_arch: &str,
) -> anyhow::Result<PythonInstallCommand> {
    match mode {
        PythonInstallMode::PinnedPlatform => Ok(PythonInstallCommand {
            program: PathBuf::from(PYTHON_INTERPRETER),
            arguments: platform_pip_arguments(manifest),
            display: format!(
                "pinned {PYTHON_INTERPRETER} / pip: install {} into {PYTHON_SITE_PACKAGES_ROOT}",
                platform_install_source(manifest)
            ),
        }),
        PythonInstallMode::ManagedLocal => Ok(PythonInstallCommand {
            program: PathBuf::new(),
            arguments: managed_uv_arguments(manifest, host_os, host_arch)?,
            display: format!(
                "managed uv {UV_VERSION} / CPython {PYTHON_RUNTIME_VERSION}: install {} into {PYTHON_SITE_PACKAGES_ROOT}",
                manifest
            ),
        }),
    }
}

fn managed_uv_arguments(
    manifest: &str,
    host_os: &str,
    host_arch: &str,
) -> anyhow::Result<Vec<OsString>> {
    if manifest == "setup.py" {
        bail!(
            "setup.py cannot be safely materialized for the Linux x86_64 runtime from {host_os}/{host_arch}; use requirements.txt or pyproject.toml, or deploy through ONREZA Cloud Builder"
        );
    }
    let mut arguments = vec![
        OsString::from("pip"),
        OsString::from("install"),
        OsString::from("--python"),
        OsString::from(PYTHON_RUNTIME_VERSION),
        OsString::from("--managed-python"),
        OsString::from("--link-mode"),
        OsString::from("copy"),
        OsString::from("--target"),
        OsString::from(PYTHON_SITE_PACKAGES_ROOT),
        OsString::from("--python-platform"),
        OsString::from(PLATFORM_PYTHON_TARGET),
        OsString::from("--only-binary"),
        OsString::from(":all:"),
    ];
    match manifest {
        "requirements.txt" | "pyproject.toml" => {
            arguments.push(OsString::from("--requirements"));
            arguments.push(OsString::from(manifest));
        }
        _ => bail!("unsupported Python dependency manifest: {manifest}"),
    }
    Ok(arguments)
}

fn platform_pip_arguments(manifest: &str) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("-m"),
        OsString::from("pip"),
        OsString::from("install"),
        OsString::from("--disable-pip-version-check"),
        OsString::from("--no-compile"),
        OsString::from("--target"),
        OsString::from(PYTHON_SITE_PACKAGES_ROOT),
    ];
    if manifest == "requirements.txt" {
        arguments.push(OsString::from("--requirement"));
        arguments.push(OsString::from(manifest));
    } else {
        arguments.push(OsString::from("."));
    }
    arguments
}

fn platform_install_source(manifest: &str) -> &str {
    if manifest == "requirements.txt" {
        manifest
    } else {
        "."
    }
}

pub(super) fn artifact_for(os: &str, arch: &str) -> anyhow::Result<UvArtifact> {
    match (os, arch) {
        ("linux", "x86_64") => Ok(UvArtifact {
            target: "x86_64-unknown-linux-musl",
            archive_name: "uv-x86_64-unknown-linux-musl.tar.gz",
            archive_sha256: "312d37f31b6f2c3bfc65668ba0efea9f1f9eaf7bc3209fe1a109e5cf861b95fa",
            binary_sha256: "907b1c5d2c1bba4111c6c2e22eeabb210eb962c4c15f5093e05cf7aec5c61b87",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("macos", "x86_64") => Ok(UvArtifact {
            target: "x86_64-apple-darwin",
            archive_name: "uv-x86_64-apple-darwin.tar.gz",
            archive_sha256: "664aed584c276f8d79cdc3b7685cd48f5d64657bd6840b06b4b2b0db731b9c99",
            binary_sha256: "4b02e5ff34bd77ce38f333bcc5f01d009e75ae93d9402cd9688871229bbb46b6",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("macos", "aarch64") => Ok(UvArtifact {
            target: "aarch64-apple-darwin",
            archive_name: "uv-aarch64-apple-darwin.tar.gz",
            archive_sha256: "82d4b99dc6ea686695b5ee142ceba03dd3e3eda2b414e94215ab7bce94972fbb",
            binary_sha256: "03d95102c0a52872ba6404c51613b732a366279e7c4d6472a59f66b1005b8295",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("windows", "x86_64") => Ok(UvArtifact {
            target: "x86_64-pc-windows-msvc",
            archive_name: "uv-x86_64-pc-windows-msvc.zip",
            archive_sha256: "4037b444541f695cd2eb93188a9346de3e334af562381411deade0a31c7bf898",
            binary_sha256: "5e559e322ad2f2e25e7d9c3cb51e3891ab0676a7e7b59ea250a021e4cb2f6e31",
            binary_name: "uv.exe",
            format: ArchiveFormat::Zip,
        }),
        _ => bail!("managed Python is not published for {os}/{arch}"),
    }
}

async fn download_archive(artifact: UvArtifact) -> anyhow::Result<Vec<u8>> {
    let url = format!(
        "{UV_RELEASE_ORIGIN}/astral-sh/uv/releases/download/{UV_VERSION}/{}",
        artifact.archive_name
    );
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(10 * 60))
        .build()
        .context("failed to create managed Python toolchain HTTP client")?;
    let response = client
        .get(&url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await
        .with_context(|| format!("failed to download managed uv {UV_VERSION}"))?
        .error_for_status()
        .with_context(|| format!("uv release server rejected {url}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_UV_ARCHIVE_BYTES)
    {
        bail!("managed uv archive exceeds the download limit");
    }

    let mut archive = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("failed while downloading managed uv")?;
        if archive.len().saturating_add(chunk.len()) > MAX_UV_ARCHIVE_BYTES as usize {
            bail!("managed uv archive exceeds the download limit");
        }
        archive.extend_from_slice(&chunk);
    }
    if sha256_hex(&archive) != artifact.archive_sha256 {
        bail!("managed uv archive does not match the CLI pin");
    }
    Ok(archive)
}

fn extract_binary(archive: &[u8], artifact: UvArtifact) -> anyhow::Result<Vec<u8>> {
    match artifact.format {
        ArchiveFormat::TarGz => extract_binary_from_tar_gz(archive, artifact.binary_name),
        ArchiveFormat::Zip => extract_binary_from_zip(archive, artifact.binary_name),
    }
}

fn extract_binary_from_tar_gz(archive: &[u8], binary_name: &str) -> anyhow::Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(archive);
    let mut tar = tar::Archive::new(decoder);
    let mut binary = None;
    for entry in tar.entries().context("failed to read managed uv archive")? {
        let entry = entry.context("invalid entry in managed uv archive")?;
        let path = entry.path().context("invalid path in managed uv archive")?;
        if path.file_name() != Some(OsStr::new(binary_name))
            || !entry.header().entry_type().is_file()
        {
            continue;
        }
        if binary.is_some() {
            bail!("managed uv archive contains duplicate binaries");
        }
        if entry.size() > MAX_UV_BINARY_BYTES {
            bail!("managed uv binary exceeds the extraction limit");
        }
        let mut bytes = Vec::new();
        entry
            .take(MAX_UV_BINARY_BYTES + 1)
            .read_to_end(&mut bytes)
            .context("failed to extract managed uv binary")?;
        if bytes.len() as u64 > MAX_UV_BINARY_BYTES {
            bail!("managed uv binary exceeds the extraction limit");
        }
        binary = Some(bytes);
    }
    binary.context("managed uv archive does not contain the expected binary")
}

#[cfg(windows)]
fn extract_binary_from_zip(archive: &[u8], binary_name: &str) -> anyhow::Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .context("failed to read managed uv zip archive")?;
    let mut binary = zip
        .by_name(binary_name)
        .context("managed uv archive does not contain the expected binary")?;
    if binary.size() > MAX_UV_BINARY_BYTES {
        bail!("managed uv binary exceeds the extraction limit");
    }
    let mut bytes = Vec::new();
    binary
        .take(MAX_UV_BINARY_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("failed to extract managed uv binary")?;
    if bytes.len() as u64 > MAX_UV_BINARY_BYTES {
        bail!("managed uv binary exceeds the extraction limit");
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn extract_binary_from_zip(_archive: &[u8], _binary_name: &str) -> anyhow::Result<Vec<u8>> {
    bail!("zip extraction is only available in the Windows nrz binary")
}

async fn cached_binary_is_valid(path: &Path, artifact: UvArtifact) -> anyhow::Result<bool> {
    let Ok(metadata) = fs::metadata(path).await else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() > MAX_UV_BINARY_BYTES {
        return Ok(false);
    }
    Ok(sha256_file(path).await? == artifact.binary_sha256)
}

async fn install_binary(path: &Path, bytes: &[u8], artifact: UvArtifact) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("managed uv cache path has no parent")?;
    fs::create_dir_all(parent)
        .await
        .with_context(|| format!("failed to create managed uv cache {}", parent.display()))?;
    let temporary_path = parent.join(format!(".uv-{}", Uuid::now_v7()));
    let mut file = fs::File::create(&temporary_path).await.with_context(|| {
        format!(
            "failed to create managed uv cache file {}",
            temporary_path.display()
        )
    })?;
    file.write_all(bytes)
        .await
        .context("failed to write managed uv cache")?;
    file.flush()
        .await
        .context("failed to flush managed uv cache")?;
    file.sync_all()
        .await
        .context("failed to persist managed uv cache")?;
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary_path, std::fs::Permissions::from_mode(0o755))
            .await
            .context("failed to make managed uv executable")?;
    }
    #[cfg(windows)]
    if fs::try_exists(path).await.unwrap_or(false) {
        let _ = fs::remove_file(path).await;
    }
    if let Err(error) = fs::rename(&temporary_path, path).await {
        let _ = fs::remove_file(&temporary_path).await;
        if !cached_binary_is_valid(path, artifact)
            .await
            .unwrap_or(false)
        {
            return Err(error)
                .with_context(|| format!("failed to install managed uv cache {}", path.display()));
        }
    }
    Ok(())
}

fn default_cache_root() -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join("Library").join("Caches"));
    }

    #[cfg(not(windows))]
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .context("HOME or USERPROFILE is required to locate the nrz Python toolchain cache")?;
    Ok(PathBuf::from(home).join(".cache"))
}

async fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("failed to verify managed uv cache {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
            encoded
        })
}
