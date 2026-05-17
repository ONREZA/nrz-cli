//! Project configuration from `onreza.toml`.

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod env_decl_tests;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

// ── Health check path configuration ─────────────────────────

/// Health check path configuration for PROCESS deployments.
///
/// - `Http(path)` — HTTP GET check at the given path (e.g. `/health`)
/// - `Tcp` — explicit opt-out from HTTP health checks (TCP port check only)
#[derive(Debug, Clone, PartialEq)]
pub enum HealthCheckPathConfig {
    /// HTTP health check at the given path.
    Http(String),
    /// Explicit TCP-only mode (user set `false` in config).
    Tcp,
}

impl Serialize for HealthCheckPathConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Http(path) => serializer.serialize_str(path),
            Self::Tcp => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for HealthCheckPathConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HcVisitor;

        impl<'de> Visitor<'de> for HcVisitor {
            type Value = HealthCheckPathConfig;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a string path like "/health" or false for TCP-only"#)
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<HealthCheckPathConfig, E> {
                if v {
                    Err(de::Error::custom(
                        "health_check_path: true is not valid, use a path string or false",
                    ))
                } else {
                    Ok(HealthCheckPathConfig::Tcp)
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<HealthCheckPathConfig, E> {
                Ok(HealthCheckPathConfig::Http(v.to_string()))
            }
        }

        deserializer.deserialize_any(HcVisitor)
    }
}

/// Top-level config loaded from `onreza.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub dev: DevSection,
    pub build: BuildSection,
    pub deploy: DeploySection,
    pub db: DbSection,
    /// Environment variable declarations: `[env]` section.
    pub env: EnvSection,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectSection {
    pub id: Option<String>,
    pub name: Option<String>,
    pub workspace: Option<String>,
    pub framework: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DevSection {
    pub command: Option<String>,
    pub port: Option<u16>,
    pub host: Option<String>,

    pub data_dir: Option<String>,

    /// Named command profiles, defined as `[dev.aliases]` in onreza.toml.
    /// Run with `nrz dev --alias <name>`.
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BuildSection {
    pub output_dirs: Option<Vec<String>>,
    pub command: Option<String>,
    pub install_command: Option<String>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DeploySection {
    /// Compute type override: "static", "isolate", "process".
    pub compute: Option<String>,
    /// Explicit entry point for PROCESS deployments (e.g. "server.ts").
    pub entry: Option<String>,
    /// Health check path for PROCESS deployments.
    /// String → HTTP check at that path; `false` → TCP only; absent → autodetect.
    pub health_check_path: Option<HealthCheckPathConfig>,
    /// Monorepo app/workspace to deploy (name, directory basename, or path).
    pub app: Option<String>,
}

/// Managed database (kaiki) configuration.
///
/// ```toml
/// [db]
/// database = "my-db"    # id or name — auto-resolved if omitted
/// branch = "dev"        # branch for nrz dev — main if omitted
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DbSection {
    /// Database ID or name. If omitted, uses first auto-inject-enabled DB.
    pub database: Option<String>,
    /// Branch name for `nrz dev`. If omitted, uses main connection.
    pub branch: Option<String>,
}

// ── Environment variable declarations ───────────────────────

/// Visibility of an environment variable on the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvVisibility {
    Plain,
    Sensitive,
}

impl EnvVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Sensitive => "sensitive",
        }
    }
}

impl fmt::Display for EnvVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Declaration of a single environment variable in `[env]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnvVarDecl {
    pub visibility: EnvVisibility,
    pub required: bool,
}

impl<'de> Deserialize<'de> for EnvVarDecl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvVarDeclVisitor;

        impl<'de> Visitor<'de> for EnvVarDeclVisitor {
            type Value = EnvVarDecl;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#""sensitive", "plain", or { visibility = "...", required = ... }"#)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<EnvVarDecl, E> {
                let visibility = EnvVisibility::deserialize(de::value::StrDeserializer::new(v))?;
                Ok(EnvVarDecl {
                    visibility,
                    required: true,
                })
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<EnvVarDecl, M::Error> {
                let mut visibility: Option<EnvVisibility> = None;
                let mut required: Option<bool> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "visibility" => {
                            visibility = Some(map.next_value::<EnvVisibility>()?);
                        }
                        "required" => {
                            required = Some(map.next_value()?);
                        }
                        other => {
                            let _: toml::Value = map.next_value()?;
                            return Err(de::Error::custom(format!(
                                "unknown field \"{other}\" in env var declaration"
                            )));
                        }
                    }
                }

                let visibility =
                    visibility.ok_or_else(|| de::Error::missing_field("visibility"))?;

                Ok(EnvVarDecl {
                    visibility,
                    required: required.unwrap_or(true),
                })
            }
        }

        deserializer.deserialize_any(EnvVarDeclVisitor)
    }
}

/// The `[env]` section: options + variable declarations.
///
/// ```toml
/// [env]
/// strict = true                    # only push declared vars
///
/// [env.declarations]
/// DATABASE_URL = "sensitive"
/// PUBLIC_API_URL = "plain"
/// OPTIONAL_VAR = { visibility = "plain", required = false }
/// ```
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct EnvSection {
    /// When true, `nrz env push` only uploads variables declared in `[env.declarations]`.
    pub strict: bool,
    /// Variable declarations keyed by name.
    pub declarations: HashMap<String, EnvVarDecl>,
}

// ── Accessor methods with defaults ──────────────────────────

impl ProjectConfig {
    pub fn merge_child(&self, child: ProjectConfig) -> ProjectConfig {
        let parent = self.clone();

        let mut aliases = parent.dev.aliases;
        aliases.extend(child.dev.aliases);

        let mut declarations = parent.env.declarations;
        declarations.extend(child.env.declarations);

        ProjectConfig {
            project: ProjectSection {
                id: merge_project_string(child.project.id, parent.project.id),
                name: merge_project_string(child.project.name, parent.project.name),
                workspace: merge_project_string(child.project.workspace, parent.project.workspace),
                framework: merge_project_string(child.project.framework, parent.project.framework),
            },
            dev: DevSection {
                command: child.dev.command.or(parent.dev.command),
                port: child.dev.port.or(parent.dev.port),
                host: child.dev.host.or(parent.dev.host),
                data_dir: child.dev.data_dir.or(parent.dev.data_dir),
                aliases,
            },
            build: BuildSection {
                output_dirs: child.build.output_dirs.or(parent.build.output_dirs),
                command: child.build.command.or(parent.build.command),
                install_command: child.build.install_command.or(parent.build.install_command),
                output_directory: child
                    .build
                    .output_directory
                    .or(parent.build.output_directory),
            },
            deploy: DeploySection {
                compute: child.deploy.compute.or(parent.deploy.compute),
                entry: child.deploy.entry.or(parent.deploy.entry),
                health_check_path: child
                    .deploy
                    .health_check_path
                    .or(parent.deploy.health_check_path),
                app: child.deploy.app.or(parent.deploy.app),
            },
            db: DbSection {
                database: child.db.database.or(parent.db.database),
                branch: child.db.branch.or(parent.db.branch),
            },
            env: EnvSection {
                strict: child.env.strict || parent.env.strict,
                declarations,
            },
        }
    }

    pub fn merge_child_for_selected_app(
        &self,
        child: ProjectConfig,
        selected_app: &str,
    ) -> ProjectConfig {
        let mut merged = self.merge_child(child);
        merged.deploy.app = Some(selected_app.to_string());
        merged
    }

    pub fn dev_alias_command(&self, name: &str) -> Option<&str> {
        self.dev.aliases.get(name).map(|s| s.as_str())
    }

    pub fn dev_port(&self) -> u16 {
        self.dev.port.unwrap_or(4321)
    }

    pub fn dev_host(&self) -> &str {
        self.dev.host.as_deref().unwrap_or("127.0.0.1")
    }

    pub fn data_dir_relative(&self) -> &str {
        self.dev.data_dir.as_deref().unwrap_or(".onreza/data")
    }

    pub fn data_dir_path(&self, project_dir: &Path) -> PathBuf {
        project_dir.join(self.data_dir_relative())
    }

    pub fn db_database(&self) -> Option<&str> {
        self.db.database.as_deref()
    }

    pub fn db_branch(&self) -> Option<&str> {
        self.db.branch.as_deref()
    }

    pub fn output_dirs(&self) -> Vec<&str> {
        match &self.build.output_dirs {
            Some(dirs) => dirs.iter().map(|s| s.as_str()).collect(),
            None => vec![
                "dist",
                ".output",
                "build",
                "out",
                "_site",
                "www",
                ".vitepress/dist",
            ],
        }
    }

    pub fn build_command(&self) -> Option<&str> {
        self.build.command.as_deref()
    }

    pub fn install_command(&self) -> Option<&str> {
        self.build.install_command.as_deref()
    }

    pub fn output_directory(&self) -> Option<&str> {
        self.build.output_directory.as_deref()
    }

    pub fn deploy_compute(&self) -> Option<&str> {
        self.deploy.compute.as_deref()
    }

    pub fn deploy_entry(&self) -> Option<&str> {
        self.deploy.entry.as_deref()
    }

    pub fn deploy_app(&self) -> Option<&str> {
        self.deploy.app.as_deref()
    }

    pub fn health_check_path(&self) -> Option<&HealthCheckPathConfig> {
        self.deploy.health_check_path.as_ref()
    }

    /// Returns keys of all required env vars declared in `[env]`.
    pub fn required_env_vars(&self) -> Vec<&str> {
        self.env
            .declarations
            .iter()
            .filter(|(_, decl)| decl.required)
            .map(|(key, _)| key.as_str())
            .collect()
    }

    /// Returns the declared visibility for a key, if declared.
    pub fn env_visibility(&self, key: &str) -> Option<EnvVisibility> {
        self.env.declarations.get(key).map(|d| d.visibility)
    }

    /// Whether `env push` should only upload variables declared in `[env]`.
    pub fn env_strict(&self) -> bool {
        self.env.strict
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildSettingSource {
    Preset,
    Detected,
    User,
}

impl BuildSettingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preset => "PRESET",
            Self::Detected => "DETECTED",
            Self::User => "USER",
        }
    }

    pub fn is_user_explicit(self) -> bool {
        self == Self::User
    }

    pub fn is_authoritative_command_absence(self) -> bool {
        matches!(self, Self::Detected | Self::User)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBuildSettings {
    pub framework_preset: Option<String>,
    pub install_command: Option<String>,
    pub install_command_source: Option<BuildSettingSource>,
    pub build_command: Option<String>,
    pub build_command_source: Option<BuildSettingSource>,
    pub output_directory: Option<String>,
    pub output_directory_source: Option<BuildSettingSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAwareSetting {
    pub value: Option<String>,
    pub source: Option<BuildSettingSource>,
    pub origin: EffectiveSettingOrigin,
}

impl SourceAwareSetting {
    fn from_user(value: Option<String>) -> Option<Self> {
        value.map(|value| Self {
            value: normalize_optional_string(Some(value)),
            source: Some(BuildSettingSource::User),
            origin: EffectiveSettingOrigin::LocalConfig,
        })
    }

    fn from_server_command(
        value: Option<String>,
        source: Option<BuildSettingSource>,
    ) -> Option<Self> {
        let value = normalize_optional_string(value);

        match source {
            Some(BuildSettingSource::Preset) => None,
            Some(source) => {
                if value.is_some() || source.is_authoritative_command_absence() {
                    Some(Self {
                        value,
                        source: Some(source),
                        origin: EffectiveSettingOrigin::ServerSettings,
                    })
                } else {
                    None
                }
            }
            None => value.map(|value| Self {
                value: Some(value),
                source: None,
                origin: EffectiveSettingOrigin::ServerSettings,
            }),
        }
    }

    fn from_server_output(
        value: Option<String>,
        source: Option<BuildSettingSource>,
    ) -> Option<Self> {
        normalize_optional_string(value).map(|value| Self {
            value: Some(value),
            source: Some(source.unwrap_or(BuildSettingSource::Preset)),
            origin: EffectiveSettingOrigin::ServerSettings,
        })
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn source_or_preset(&self) -> BuildSettingSource {
        self.source.unwrap_or(BuildSettingSource::Preset)
    }

    pub fn origin(&self) -> EffectiveSettingOrigin {
        self.origin
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveSettingOrigin {
    Cli,
    LocalConfig,
    ServerSettings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfigValue {
    pub value: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfigList {
    pub values: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfigExplanation {
    pub project_dir: String,
    pub project_id: EffectiveConfigValue,
    pub framework: EffectiveConfigValue,
    pub install_command: EffectiveConfigValue,
    pub build_command: EffectiveConfigValue,
    pub output_directory: EffectiveConfigValue,
    pub output_dirs: EffectiveConfigList,
    pub deploy_compute: EffectiveConfigValue,
    pub deploy_entry: EffectiveConfigValue,
    pub deploy_app: EffectiveConfigValue,
}

#[derive(Debug, Clone)]
pub struct EffectiveProjectConfig {
    project_dir: PathBuf,
    config: ProjectConfig,
    project_id: Option<String>,
    project_id_source: Option<EffectiveSettingOrigin>,
    framework_override: Option<String>,
    framework_override_source: Option<EffectiveSettingOrigin>,
    deploy_app: Option<String>,
    deploy_app_source: Option<EffectiveSettingOrigin>,
    install_command: Option<SourceAwareSetting>,
    build_command: Option<SourceAwareSetting>,
    output_directory: Option<SourceAwareSetting>,
}

impl EffectiveProjectConfig {
    pub fn from_project_config(project_dir: PathBuf, config: ProjectConfig) -> Self {
        let project_id = normalize_optional_string(config.project.id.clone());
        let project_id_source = project_id
            .as_ref()
            .map(|_| EffectiveSettingOrigin::LocalConfig);
        let framework_override =
            normalize_authoritative_framework(config.project.framework.as_deref())
                .map(str::to_string);
        let framework_override_source = framework_override
            .as_ref()
            .map(|_| EffectiveSettingOrigin::LocalConfig);
        let deploy_app = normalize_optional_string(config.deploy.app.clone());
        let deploy_app_source = deploy_app
            .as_ref()
            .map(|_| EffectiveSettingOrigin::LocalConfig);
        let install_command = SourceAwareSetting::from_user(config.build.install_command.clone());
        let build_command = SourceAwareSetting::from_user(config.build.command.clone());
        let output_directory = normalize_optional_string(config.build.output_directory.clone())
            .map(|value| SourceAwareSetting {
                value: Some(value),
                source: Some(BuildSettingSource::User),
                origin: EffectiveSettingOrigin::LocalConfig,
            });

        Self {
            project_dir,
            config,
            project_id,
            project_id_source,
            framework_override,
            framework_override_source,
            deploy_app,
            deploy_app_source,
            install_command,
            build_command,
            output_directory,
        }
    }

    pub fn load(project_dir: PathBuf) -> anyhow::Result<Self> {
        let config = load(&project_dir)?;
        Ok(Self::from_project_config(project_dir, config))
    }

    pub fn apply_project_id_override(&mut self, project_id: Option<&str>) -> anyhow::Result<()> {
        let Some(project_id) = project_id else {
            return Ok(());
        };
        let Some(project_id) = normalize_optional_string(Some(project_id.to_string())) else {
            anyhow::bail!("--project-id must not be empty");
        };

        self.project_id = Some(project_id);
        self.project_id_source = Some(EffectiveSettingOrigin::Cli);
        Ok(())
    }

    pub fn apply_deploy_app_cli_override(
        &mut self,
        deploy_app: Option<&str>,
    ) -> anyhow::Result<()> {
        let Some(deploy_app) = deploy_app else {
            return Ok(());
        };
        let Some(deploy_app) = normalize_optional_string(Some(deploy_app.to_string())) else {
            anyhow::bail!("--app must not be empty");
        };

        self.deploy_app = Some(deploy_app);
        self.deploy_app_source = Some(EffectiveSettingOrigin::Cli);
        Ok(())
    }

    pub fn apply_server_settings(&mut self, settings: Option<&ProjectBuildSettings>) {
        let Some(settings) = settings else {
            return;
        };

        if self.framework_override.is_none()
            && let Some(framework) =
                normalize_authoritative_framework(settings.framework_preset.as_deref())
        {
            self.framework_override = Some(framework.to_string());
            self.framework_override_source = Some(EffectiveSettingOrigin::ServerSettings);
        }

        if self.install_command.is_none() {
            self.install_command = SourceAwareSetting::from_server_command(
                settings.install_command.clone(),
                settings.install_command_source,
            );
        }

        if self.build_command.is_none() {
            self.build_command = SourceAwareSetting::from_server_command(
                settings.build_command.clone(),
                settings.build_command_source,
            );
        }

        if self.output_directory.is_none() {
            self.output_directory = SourceAwareSetting::from_server_output(
                settings.output_directory.clone(),
                settings.output_directory_source,
            );
        }
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    pub fn framework_override(&self) -> Option<&str> {
        self.framework_override.as_deref()
    }

    pub fn install_command(&self) -> Option<&SourceAwareSetting> {
        self.install_command.as_ref()
    }

    pub fn build_command(&self) -> Option<&SourceAwareSetting> {
        self.build_command.as_ref()
    }

    pub fn output_directory(&self) -> Option<&SourceAwareSetting> {
        self.output_directory.as_ref()
    }

    pub fn output_dirs(&self) -> Vec<&str> {
        self.config.output_dirs()
    }

    pub fn deploy_compute(&self) -> Option<&str> {
        self.config.deploy_compute()
    }

    pub fn deploy_entry(&self) -> Option<&str> {
        self.config.deploy_entry()
    }

    pub fn deploy_app(&self) -> Option<&str> {
        self.deploy_app.as_deref()
    }

    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    pub fn explain(&self) -> EffectiveConfigExplanation {
        EffectiveConfigExplanation {
            project_dir: self.project_dir.display().to_string(),
            project_id: explain_origin_value(self.project_id(), self.project_id_source, "absent"),
            framework: explain_framework(
                self.framework_override.as_deref(),
                self.framework_override_source,
            ),
            install_command: explain_source_aware_setting(self.install_command.as_ref(), "auto"),
            build_command: explain_source_aware_setting(self.build_command.as_ref(), "auto"),
            output_directory: explain_source_aware_setting(self.output_directory.as_ref(), "auto"),
            output_dirs: EffectiveConfigList {
                values: self.output_dirs().into_iter().map(str::to_string).collect(),
                source: if self.config.build.output_dirs.is_some() {
                    "onreza.toml".to_string()
                } else {
                    "default".to_string()
                },
            },
            deploy_compute: explain_config_option(self.deploy_compute(), "onreza.toml", "auto"),
            deploy_entry: explain_config_option(self.deploy_entry(), "onreza.toml", "absent"),
            deploy_app: explain_origin_value(self.deploy_app(), self.deploy_app_source, "absent"),
        }
    }
}

fn merge_project_string(child: Option<String>, parent: Option<String>) -> Option<String> {
    normalize_optional_string(child).or_else(|| normalize_optional_string(parent))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn explain_config_option(
    value: Option<&str>,
    present_source: &str,
    absent_source: &str,
) -> EffectiveConfigValue {
    EffectiveConfigValue {
        value: value.map(str::to_string),
        source: if value.is_some() {
            present_source.to_string()
        } else {
            absent_source.to_string()
        },
    }
}

fn explain_framework(
    value: Option<&str>,
    source: Option<EffectiveSettingOrigin>,
) -> EffectiveConfigValue {
    explain_origin_value(value, source, "auto")
}

fn explain_origin_value(
    value: Option<&str>,
    source: Option<EffectiveSettingOrigin>,
    absent_source: &str,
) -> EffectiveConfigValue {
    EffectiveConfigValue {
        value: value.map(str::to_string),
        source: source
            .map(explain_effective_origin)
            .unwrap_or_else(|| absent_source.to_string()),
    }
}

fn explain_source_aware_setting(
    setting: Option<&SourceAwareSetting>,
    absent_source: &str,
) -> EffectiveConfigValue {
    let Some(setting) = setting else {
        return EffectiveConfigValue {
            value: None,
            source: absent_source.to_string(),
        };
    };

    EffectiveConfigValue {
        value: setting.value().map(str::to_string),
        source: explain_source_aware_origin(setting),
    }
}

fn explain_source_aware_origin(setting: &SourceAwareSetting) -> String {
    match setting.origin() {
        EffectiveSettingOrigin::Cli => "cli".to_string(),
        EffectiveSettingOrigin::LocalConfig => "onreza.toml".to_string(),
        EffectiveSettingOrigin::ServerSettings => match setting.source {
            Some(source) => format!("server:{}", source.as_str()),
            None => "server".to_string(),
        },
    }
}

fn explain_effective_origin(source: EffectiveSettingOrigin) -> String {
    match source {
        EffectiveSettingOrigin::Cli => "cli".to_string(),
        EffectiveSettingOrigin::LocalConfig => "onreza.toml".to_string(),
        EffectiveSettingOrigin::ServerSettings => "server".to_string(),
    }
}

pub fn normalize_authoritative_framework(framework: Option<&str>) -> Option<&str> {
    let framework = framework?.trim();
    if framework.is_empty() || framework.eq_ignore_ascii_case("other") {
        return None;
    }
    Some(framework)
}

// ── Load / Save ─────────────────────────────────────────────

const CONFIG_FILENAME: &str = "onreza.toml";

/// Load `onreza.toml` from project directory. Returns `Default` if file not found.
pub fn load(project_dir: &Path) -> anyhow::Result<ProjectConfig> {
    let path = project_dir.join(CONFIG_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let config: ProjectConfig = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            // Validate [deploy] entry: must be non-empty relative path
            if let Some(ref entry) = config.deploy.entry {
                if entry.is_empty() {
                    anyhow::bail!("[deploy] entry must not be empty");
                }
                if entry.starts_with('/') {
                    anyhow::bail!("[deploy] entry must be a relative path, got: \"{entry}\"");
                }
                if entry.contains("..") {
                    anyhow::bail!("[deploy] entry must not contain \"..\", got: \"{entry}\"");
                }
            }
            // Validate [deploy] health_check_path
            if let Some(HealthCheckPathConfig::Http(ref path)) = config.deploy.health_check_path {
                if !path.starts_with('/') {
                    anyhow::bail!(
                        "[deploy] health_check_path must start with '/', got: \"{path}\""
                    );
                }
                if path.contains("..") {
                    anyhow::bail!(
                        "[deploy] health_check_path must not contain '..', got: \"{path}\""
                    );
                }
                if path.contains('?') || path.contains('#') {
                    anyhow::bail!(
                        "[deploy] health_check_path must not contain query or fragment, got: \"{path}\""
                    );
                }
            }
            // Validate env var names: must match ^[A-Z][A-Z0-9_]*$
            for key in config.env.declarations.keys() {
                let valid = !key.is_empty()
                    && key.as_bytes()[0].is_ascii_uppercase()
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
                if !valid {
                    anyhow::bail!(
                        "invalid env var name \"{}\" in [env.declarations]: \
                         must be UPPER_SNAKE_CASE (start with A-Z, contain only A-Z, 0-9, _)",
                        key
                    );
                }
            }
            // Validate [db] section
            if let Some(ref db) = config.db.database
                && db.trim().is_empty()
            {
                anyhow::bail!("[db] database must not be empty");
            }
            if let Some(ref branch) = config.db.branch
                && branch.trim().is_empty()
            {
                anyhow::bail!("[db] branch must not be empty");
            }
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProjectConfig::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

/// Generate a template `onreza.toml` with commented-out defaults.
///
/// If `project_id` is `None`, the template is created without a project ID
/// (local-only scaffold before platform linking).
pub fn generate_template(
    project_id: Option<&str>,
    project_name: Option<&str>,
    workspace_slug: Option<&str>,
) -> String {
    let mut project_lines = String::from(
        "#:schema https://raw.githubusercontent.com/onreza/nrz-cli/main/onreza.schema.json\n\n[project]\n",
    );
    if let Some(id) = project_id {
        let id = escape_toml_value(id);
        project_lines.push_str(&format!("id = \"{id}\"\n"));
    } else {
        project_lines.push_str("# id = \"\"\n");
    }
    if let Some(name) = project_name {
        let name = escape_toml_value(name);
        project_lines.push_str(&format!("name = \"{name}\"\n"));
    } else {
        project_lines.push_str("# name = \"\"\n");
    }
    if let Some(ws) = workspace_slug {
        let ws = escape_toml_value(ws);
        project_lines.push_str(&format!("workspace = \"{ws}\"\n"));
    }

    format!(
        r#"{project_lines}
# [dev]
# command = ""
# port = 4321
# host = "127.0.0.1"

# data_dir = ".onreza/data"

# [dev.aliases]
# network = "npm run dev -- --host 0.0.0.0"
# staging = "npm run dev -- --host 0.0.0.0 --port 3001"

# [build]
# install_command = "npm install"
# command = "npm run build"
# output_directory = "dist"
# output_dirs = ["dist", ".output", "build", "out", "_site", "www", ".vitepress/dist"]

# [deploy]
# compute = "static"    # "static", "isolate", or "process"
# entry = "server.ts"   # entry point for PROCESS deployments
# health_check_path = "/health"  # HTTP health check path, or false for TCP only

# [db]
# database = ""          # managed database id or name (auto-resolved if omitted)
# branch = ""            # branch for nrz dev (main if omitted)

# [env]
# strict = false

# [env.declarations]
# DATABASE_URL = "sensitive"
# PUBLIC_API_URL = "plain"
# OPTIONAL_VAR = {{ visibility = "plain", required = false }}
"#
    )
}

/// Create or update `onreza.toml`.
///
/// If the file exists, update `[project]` fields in-place preserving other content.
/// If the file doesn't exist, generate from template.
pub fn save_or_update(
    project_dir: &Path,
    project_id: &str,
    project_name: Option<&str>,
    workspace_slug: Option<&str>,
) -> anyhow::Result<()> {
    let path = project_dir.join(CONFIG_FILENAME);

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        let id_matches = config.project.id.as_deref() == Some(project_id);
        let name_matches = project_name.is_none() || config.project.name.as_deref() == project_name;
        let ws_matches =
            workspace_slug.is_none() || config.project.workspace.as_deref() == workspace_slug;

        if id_matches && name_matches && ws_matches {
            return Ok(());
        }

        // Update fields in-place preserving comments and formatting
        let updated =
            update_project_fields_in_toml(&content, project_id, project_name, workspace_slug);
        std::fs::write(&path, updated)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        let content = generate_template(Some(project_id), project_name, workspace_slug);
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

/// Save detected framework slug to `onreza.toml` `[project]` section.
///
/// If the file exists, updates `framework` field in-place.
/// If not, does nothing (scaffold must exist first).
pub fn save_framework(project_dir: &Path, framework: &str) -> anyhow::Result<bool> {
    let path = project_dir.join(CONFIG_FILENAME);
    if !path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let config: ProjectConfig =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;

    if config.project.framework.as_deref() == Some(framework) {
        return Ok(true);
    }

    let updated = update_single_field_in_toml(&content, "framework", framework);
    std::fs::write(&path, updated)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(true)
}

/// Update a single field in the `[project]` section in-place.
fn update_single_field_in_toml(content: &str, key: &str, value: &str) -> String {
    let escaped = escape_toml_value(value);
    let mut result = String::new();
    let mut in_project_section = false;
    let mut field_replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            if in_project_section && !field_replaced {
                result.push_str(&format!("{key} = \"{escaped}\"\n"));
                field_replaced = true;
            }
            in_project_section = trimmed == "[project]";
        }

        if in_project_section
            && !field_replaced
            && let Some(replaced) = try_replace_field(trimmed, key, &escaped)
        {
            result.push_str(&replaced);
            result.push('\n');
            field_replaced = true;
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    if in_project_section && !field_replaced {
        result.push_str(&format!("{key} = \"{escaped}\"\n"));
    }

    result
}

/// Resolve project ID from explicit flag, config, or fail.
pub fn resolve_project_id(
    explicit: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    if let Some(id) = explicit
        && !id.is_empty()
    {
        return Ok(id.to_string());
    }

    if let Some(id) = &config.project.id
        && !id.is_empty()
    {
        return Ok(id.clone());
    }

    anyhow::bail!(
        "no project specified. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
    );
}

/// Update `[project]` fields in-place, preserving the rest of the file content
/// including comments.
fn update_project_fields_in_toml(
    content: &str,
    new_id: &str,
    new_name: Option<&str>,
    new_workspace: Option<&str>,
) -> String {
    let escaped_id = escape_toml_value(new_id);
    let escaped_name = new_name.map(escape_toml_value);
    let escaped_ws = new_workspace.map(escape_toml_value);

    let mut result = String::new();
    let mut in_project_section = false;
    let mut id_replaced = false;
    let mut name_replaced = false;
    let mut workspace_replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            // Before leaving [project] section, insert any missing fields
            if in_project_section {
                if !id_replaced {
                    result.push_str(&format!("id = \"{escaped_id}\"\n"));
                    id_replaced = true;
                }
                if !name_replaced && let Some(ref name) = escaped_name {
                    result.push_str(&format!("name = \"{name}\"\n"));
                    name_replaced = true;
                }
                if !workspace_replaced && let Some(ref ws) = escaped_ws {
                    result.push_str(&format!("workspace = \"{ws}\"\n"));
                    workspace_replaced = true;
                }
            }
            in_project_section = trimmed == "[project]";
        }

        if in_project_section {
            if let Some(replaced) = try_replace_field(trimmed, "id", &escaped_id)
                && !id_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                id_replaced = true;
                continue;
            }
            if let Some(ref name) = escaped_name
                && let Some(replaced) = try_replace_field(trimmed, "name", name)
                && !name_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                name_replaced = true;
                continue;
            }
            if let Some(ref ws) = escaped_ws
                && let Some(replaced) = try_replace_field(trimmed, "workspace", ws)
                && !workspace_replaced
            {
                result.push_str(&replaced);
                result.push('\n');
                workspace_replaced = true;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Handle end-of-file: if still in [project] section, insert missing fields
    if in_project_section {
        if !id_replaced {
            result.push_str(&format!("id = \"{escaped_id}\"\n"));
            id_replaced = true;
        }
        if !name_replaced && let Some(ref name) = escaped_name {
            result.push_str(&format!("name = \"{name}\"\n"));
        }
        if !workspace_replaced && let Some(ref ws) = escaped_ws {
            result.push_str(&format!("workspace = \"{ws}\"\n"));
        }
    }

    // If we never found [project] section
    if !id_replaced {
        let has_project_section = result.lines().any(|l| l.trim() == "[project]");
        if !has_project_section {
            let mut header = format!("[project]\nid = \"{escaped_id}\"\n");
            if let Some(ref name) = escaped_name {
                header.push_str(&format!("name = \"{name}\"\n"));
            }
            if let Some(ref ws) = escaped_ws {
                header.push_str(&format!("workspace = \"{ws}\"\n"));
            }
            header.push('\n');
            return format!("{header}{result}");
        }
        let mut final_result = String::new();
        for line in result.lines() {
            final_result.push_str(line);
            final_result.push('\n');
            if line.trim() == "[project]" {
                final_result.push_str(&format!("id = \"{escaped_id}\"\n"));
                if let Some(ref name) = escaped_name {
                    final_result.push_str(&format!("name = \"{name}\"\n"));
                }
                if let Some(ref ws) = escaped_ws {
                    final_result.push_str(&format!("workspace = \"{ws}\"\n"));
                }
            }
        }
        return final_result;
    }

    result
}

/// Try to match and replace a `key = "..."` or `# key = "..."` line.
/// Returns `Some(replacement)` if matched (handles both active and commented-out lines).
fn try_replace_field(trimmed: &str, key: &str, value: &str) -> Option<String> {
    // Strip leading `# ` for commented-out lines
    let effective = trimmed
        .strip_prefix('#')
        .map(|s| s.trim_start())
        .unwrap_or(trimmed);

    if !effective.starts_with(key) {
        return None;
    }
    let rest = effective.strip_prefix(key)?;
    let rest = rest.trim_start();
    if rest.starts_with('=') {
        Some(format!("{key} = \"{value}\""))
    } else {
        None
    }
}

/// Escape a string value for safe TOML insertion.
fn escape_toml_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
