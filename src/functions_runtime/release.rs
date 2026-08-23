use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;
use uuid::Uuid;

const PROTOCOL_VERSION: &str = "onreza-functions-poc/v1";
const RELEASE_ORIGIN: &str = "https://releases.onreza.ru";
const PINNED_RUNTIME_RELEASE_ID: &str = "runtime-49b71f869c9cf9c3f8e83a1306f57e0425cdc267";
const PINNED_MANIFEST_URL: &str = "https://releases.onreza.ru/releases/runtime-49b71f869c9cf9c3f8e83a1306f57e0425cdc267/manifest.json";
const PINNED_MANIFEST_SHA256: &str =
    "730b51b9fe1fe3409440ee67c491355c5d73b6db1843ae28af97f26d123ecca5";
const PINNED_SIGNATURE_URL: &str = "https://releases.onreza.ru/releases/runtime-49b71f869c9cf9c3f8e83a1306f57e0425cdc267/manifest.sig";
const SIGNING_PUBLIC_KEY_PEM: &str =
    include_str!("../../assets/functions-runtime-signing-public.pem");

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CachedRuntime {
    pub(crate) runtime_release_id: String,
    pub(crate) target: String,
    pub(crate) path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeStatus {
    pub(crate) runtime_release_id: String,
    pub(crate) target: String,
    pub(crate) path: PathBuf,
    pub(crate) installed: bool,
}

pub(crate) struct RuntimeResolver {
    config: ResolverConfig,
    client: reqwest::Client,
}

#[derive(Clone)]
struct ResolverConfig {
    runtime_release_id: String,
    manifest_url: Url,
    manifest_sha256: String,
    signature_url: Url,
    verifying_key: VerifyingKey,
    cache_root: PathBuf,
    require_release_origin: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeManifest {
    schema_version: u32,
    runtime_release_id: String,
    protocol_version: String,
    source: RuntimeSource,
    artifacts: Vec<RuntimeArtifact>,
}

#[derive(Debug, Deserialize)]
struct RuntimeSource {
    revision: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeArtifact {
    target: String,
    file_name: String,
    sha256: String,
    size_bytes: u64,
}

impl RuntimeResolver {
    pub(crate) fn pinned() -> anyhow::Result<Self> {
        Self::from_config(ResolverConfig {
            runtime_release_id: PINNED_RUNTIME_RELEASE_ID.to_string(),
            manifest_url: Url::parse(PINNED_MANIFEST_URL)
                .context("invalid pinned runtime manifest URL")?,
            manifest_sha256: PINNED_MANIFEST_SHA256.to_string(),
            signature_url: Url::parse(PINNED_SIGNATURE_URL)
                .context("invalid pinned runtime signature URL")?,
            verifying_key: VerifyingKey::from_public_key_pem(SIGNING_PUBLIC_KEY_PEM)
                .context("invalid embedded Functions runtime signing key")?,
            cache_root: default_cache_root()?.join("nrz").join("functions-runtime"),
            require_release_origin: true,
        })
    }

    fn from_config(config: ResolverConfig) -> anyhow::Result<Self> {
        if config.require_release_origin {
            validate_release_url(&config.manifest_url)?;
            validate_release_url(&config.signature_url)?;
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10 * 60))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("failed to create Functions runtime HTTP client")?;
        Ok(Self { config, client })
    }

    pub(crate) async fn resolve(&self) -> anyhow::Result<CachedRuntime> {
        let target = runtime_target()?;
        let cached_path = self.cached_path(target);
        if let Some(artifact) = self.cached_manifest_artifact(target).await?
            && self
                .cached_runtime_is_valid(&cached_path, &artifact)
                .await?
        {
            return Ok(self.cached_runtime(target, cached_path));
        }

        let manifest_bytes = self.download_bytes(&self.config.manifest_url).await?;
        let signature_bytes = self.download_bytes(&self.config.signature_url).await?;
        let artifact = self.verify_manifest(&manifest_bytes, &signature_bytes, target)?;
        self.cache_release_metadata(&manifest_bytes, &signature_bytes)
            .await?;
        let artifact_url = artifact_url(&self.config.manifest_url, &artifact.file_name)?;
        if self.config.require_release_origin {
            validate_release_url(&artifact_url)?;
        }
        if self
            .cached_runtime_is_valid(&cached_path, &artifact)
            .await?
        {
            return Ok(self.cached_runtime(target, cached_path));
        }

        self.download_artifact(&artifact_url, &cached_path, &artifact)
            .await?;
        Ok(self.cached_runtime(target, cached_path))
    }

    pub(crate) async fn status(&self) -> anyhow::Result<RuntimeStatus> {
        let target = runtime_target()?;
        let path = self.cached_path(target);
        let installed = match self.cached_manifest_artifact(target).await? {
            Some(artifact) => self.cached_runtime_is_valid(&path, &artifact).await?,
            None => false,
        };
        Ok(RuntimeStatus {
            runtime_release_id: self.config.runtime_release_id.clone(),
            target: target.to_string(),
            path,
            installed,
        })
    }

    fn cached_runtime(&self, target: &str, path: PathBuf) -> CachedRuntime {
        CachedRuntime {
            runtime_release_id: self.config.runtime_release_id.clone(),
            target: target.to_string(),
            path,
        }
    }

    fn cached_path(&self, target: &str) -> PathBuf {
        self.config
            .cache_root
            .join(&self.config.runtime_release_id)
            .join(target)
            .join(runtime_file_name(target))
    }

    async fn cached_manifest_artifact(
        &self,
        target: &str,
    ) -> anyhow::Result<Option<RuntimeArtifact>> {
        let release_root = self.config.cache_root.join(&self.config.runtime_release_id);
        let manifest = match fs::read(release_root.join("manifest.json")).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("failed to read cached runtime manifest"),
        };
        let signature = match fs::read(release_root.join("manifest.sig")).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("failed to read cached runtime signature"),
        };
        Ok(self.verify_manifest(&manifest, &signature, target).ok())
    }

    fn verify_manifest(
        &self,
        manifest_bytes: &[u8],
        signature_bytes: &[u8],
        target: &str,
    ) -> anyhow::Result<RuntimeArtifact> {
        if sha256_hex(manifest_bytes) != self.config.manifest_sha256 {
            bail!("Functions runtime manifest digest does not match the CLI pin");
        }
        let signature = Signature::from_slice(signature_bytes)
            .context("Functions runtime manifest signature is not Ed25519")?;
        self.config
            .verifying_key
            .verify(manifest_bytes, &signature)
            .context("Functions runtime manifest signature verification failed")?;
        let manifest: RuntimeManifest = serde_json::from_slice(manifest_bytes)
            .context("failed to parse signed Functions runtime manifest")?;
        validate_manifest(&manifest, &self.config.runtime_release_id, target).cloned()
    }

    async fn cache_release_metadata(
        &self,
        manifest: &[u8],
        signature: &[u8],
    ) -> anyhow::Result<()> {
        let release_root = self.config.cache_root.join(&self.config.runtime_release_id);
        fs::create_dir_all(&release_root).await.with_context(|| {
            format!("failed to create runtime cache {}", release_root.display())
        })?;
        write_atomic(&release_root.join("manifest.json"), manifest)
            .await
            .context("failed to cache signed runtime manifest")?;
        write_atomic(&release_root.join("manifest.sig"), signature)
            .await
            .context("failed to cache runtime manifest signature")
    }

    async fn download_bytes(&self, url: &Url) -> anyhow::Result<Vec<u8>> {
        self.client
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("failed to download {url}"))?
            .error_for_status()
            .with_context(|| format!("runtime release server rejected {url}"))?
            .bytes()
            .await
            .with_context(|| format!("failed to read {url}"))
            .map(|bytes| bytes.to_vec())
    }

    async fn download_artifact(
        &self,
        url: &Url,
        cached_path: &Path,
        artifact: &RuntimeArtifact,
    ) -> anyhow::Result<()> {
        let parent = cached_path
            .parent()
            .context("Functions runtime cache path has no parent")?;
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create runtime cache {}", parent.display()))?;
        let temporary_path = parent.join(format!(".download-{}", Uuid::now_v7()));
        let result = self
            .download_artifact_to(url, &temporary_path, artifact)
            .await;
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary_path).await;
            return Err(error);
        }

        if self.cached_runtime_is_valid(cached_path, artifact).await? {
            let _ = fs::remove_file(&temporary_path).await;
            return Ok(());
        }
        if let Err(error) = set_executable(&temporary_path).await {
            let _ = fs::remove_file(&temporary_path).await;
            return Err(error);
        }
        #[cfg(windows)]
        if fs::try_exists(cached_path).await.unwrap_or(false) {
            let _ = fs::remove_file(cached_path).await;
        }
        match fs::rename(&temporary_path, cached_path).await {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary_path).await;
                if self
                    .cached_runtime_is_valid(cached_path, artifact)
                    .await
                    .unwrap_or(false)
                {
                    Ok(())
                } else {
                    Err(error).with_context(|| {
                        format!("failed to install runtime cache {}", cached_path.display())
                    })
                }
            }
        }
    }

    async fn download_artifact_to(
        &self,
        url: &Url,
        path: &Path,
        artifact: &RuntimeArtifact,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("failed to download Functions runtime {url}"))?
            .error_for_status()
            .with_context(|| format!("runtime release server rejected {url}"))?;
        let mut stream = response.bytes_stream();
        let mut file = fs::File::create(path)
            .await
            .with_context(|| format!("failed to create runtime download {}", path.display()))?;
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("failed while streaming Functions runtime")?;
            size += chunk.len() as u64;
            if size > artifact.size_bytes {
                bail!("downloaded Functions runtime exceeds its signed size");
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .context("failed to write Functions runtime cache")?;
        }
        file.flush()
            .await
            .context("failed to flush Functions runtime cache")?;
        file.sync_all()
            .await
            .context("failed to persist Functions runtime cache")?;
        if size != artifact.size_bytes || hex_digest(hasher.finalize()) != artifact.sha256 {
            bail!("downloaded Functions runtime does not match its signed manifest");
        }
        Ok(())
    }

    async fn cached_runtime_is_valid(
        &self,
        path: &Path,
        expected: &RuntimeArtifact,
    ) -> anyhow::Result<bool> {
        let Ok(metadata) = fs::metadata(path).await else {
            return Ok(false);
        };
        if !metadata.is_file() {
            return Ok(false);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if metadata.permissions().mode() & 0o111 == 0 {
                return Ok(false);
            }
        }
        if metadata.len() != expected.size_bytes {
            return Ok(false);
        }
        Ok(sha256_file(path).await? == expected.sha256)
    }
}

fn validate_manifest<'a>(
    manifest: &'a RuntimeManifest,
    expected_release_id: &str,
    target: &str,
) -> anyhow::Result<&'a RuntimeArtifact> {
    if manifest.schema_version != 1
        || manifest.runtime_release_id != expected_release_id
        || manifest.protocol_version != PROTOCOL_VERSION
        || format!("runtime-{}", manifest.source.revision) != expected_release_id
    {
        bail!("signed Functions runtime manifest does not match the CLI runtime contract");
    }
    let mut artifacts = manifest
        .artifacts
        .iter()
        .filter(|item| item.target == target);
    let artifact = artifacts.next().with_context(|| {
        format!("signed Functions runtime manifest has no artifact for {target}")
    })?;
    if artifacts.next().is_some() {
        bail!("signed Functions runtime manifest has duplicate artifacts for {target}");
    }
    if artifact.file_name != runtime_file_name(target)
        || Path::new(&artifact.file_name).file_name() != Some(OsStr::new(&artifact.file_name))
        || artifact.sha256.len() != 64
        || !artifact
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || artifact.size_bytes == 0
    {
        bail!("signed Functions runtime artifact metadata is invalid");
    }
    Ok(artifact)
}

fn artifact_url(manifest_url: &Url, file_name: &str) -> anyhow::Result<Url> {
    manifest_url
        .join(file_name)
        .context("failed to resolve Functions runtime artifact URL")
}

fn validate_release_url(url: &Url) -> anyhow::Result<()> {
    let origin = Url::parse(RELEASE_ORIGIN).expect("release origin constant must be valid");
    if url.scheme() != "https"
        || url.host_str() != origin.host_str()
        || url.port_or_known_default() != origin.port_or_known_default()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("Functions runtime release URL is outside the trusted release origin");
    }
    Ok(())
}

fn runtime_target() -> anyhow::Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("windows", "x86_64") => Ok("windows-x64"),
        (os, arch) => bail!("ONREZA Functions runtime is not published for {os}/{arch}"),
    }
}

fn runtime_file_name(target: &str) -> String {
    let suffix = if target == "windows-x64" { ".exe" } else { "" };
    format!("onreza-functions-runtime-{target}{suffix}")
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
        .context("HOME or USERPROFILE is required to locate the nrz runtime cache")?;
    Ok(PathBuf::from(home).join(".cache"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

async fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = fs::File::open(path)
        .await
        .with_context(|| format!("failed to open runtime cache {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .context("failed to verify Functions runtime cache")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize()))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = digest.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

async fn set_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .await
            .with_context(|| format!("failed to make runtime executable: {}", path.display()))?;
    }
    Ok(())
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("runtime cache metadata path has no parent")?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .context("runtime cache metadata path has no file name")?;
    let temporary_path = parent.join(format!(".{name}-{}", Uuid::now_v7()));
    let mut file = fs::File::create(&temporary_path).await.with_context(|| {
        format!(
            "failed to create runtime metadata {}",
            temporary_path.display()
        )
    })?;
    file.write_all(bytes)
        .await
        .context("failed to write runtime metadata")?;
    file.flush()
        .await
        .context("failed to flush runtime metadata")?;
    file.sync_all()
        .await
        .context("failed to persist runtime metadata")?;
    drop(file);

    #[cfg(windows)]
    if fs::try_exists(path).await.unwrap_or(false) {
        match fs::read(path).await {
            Ok(existing) if existing == bytes => {
                let _ = fs::remove_file(&temporary_path).await;
                return Ok(());
            }
            _ => {
                let _ = fs::remove_file(path).await;
            }
        }
    }

    match fs::rename(&temporary_path, path).await {
        Ok(()) => Ok(()),
        Err(error) => {
            let matches = fs::read(path).await.is_ok_and(|existing| existing == bytes);
            let _ = fs::remove_file(&temporary_path).await;
            if matches {
                Ok(())
            } else {
                Err(error).with_context(|| {
                    format!("failed to install runtime metadata {}", path.display())
                })
            }
        }
    }
}

#[cfg(test)]
impl RuntimeResolver {
    pub(super) fn for_test(
        runtime_release_id: &str,
        manifest_url: Url,
        manifest_sha256: String,
        signature_url: Url,
        verifying_key: VerifyingKey,
        cache_root: PathBuf,
    ) -> anyhow::Result<Self> {
        Self::from_config(ResolverConfig {
            runtime_release_id: runtime_release_id.to_string(),
            manifest_url,
            manifest_sha256,
            signature_url,
            verifying_key,
            cache_root,
            require_release_origin: false,
        })
    }
}
