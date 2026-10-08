use super::{
    EffectiveConfigExplanation, EffectiveConfigList, EffectiveConfigValue, EffectiveProjectConfig,
    EffectiveSettingOrigin, SourceAwareSetting,
};

impl EffectiveProjectConfig {
    pub fn explain(&self) -> anyhow::Result<EffectiveConfigExplanation> {
        let detection = crate::detect::detect_with_framework_override(
            &self.project_dir,
            self.framework_override(),
        );
        let build_toolchain =
            crate::detect::application_runtime::resolve_build_toolchain(&detection, &self.config)?;
        let build_python_version = build_toolchain.resolved_python_minor();
        let selected_serving_python_version =
            crate::detect::application_runtime::resolve_serving_python_minor(
                &detection,
                &self.config,
                &build_toolchain,
                None,
            )?;
        let deploy_python_version = self
            .config
            .deploy
            .python_version
            .or(selected_serving_python_version);
        Ok(EffectiveConfigExplanation {
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
            build_toolchain: explain_config_option(
                self.config
                    .build
                    .selected_toolchain_family()
                    .map(|family| match family {
                        nrz_source_bundle::BuildToolchainFamily::Node => "node",
                        nrz_source_bundle::BuildToolchainFamily::Bun => "bun",
                        nrz_source_bundle::BuildToolchainFamily::Python => "python",
                        nrz_source_bundle::BuildToolchainFamily::Native => "native",
                    }),
                "onreza.toml",
                "auto",
            ),
            build_python_version: explain_config_option(
                build_python_version.map(nrz_source_bundle::PythonMinor::version),
                if self.config.build.python_version.is_some()
                    || self.config.deploy.python_version.is_some()
                {
                    "onreza.toml"
                } else {
                    "default"
                },
                "default",
            ),
            deploy_compute: explain_config_option(self.deploy_compute(), "onreza.toml", "auto"),
            deploy_entry: explain_config_option(self.deploy_entry(), "onreza.toml", "absent"),
            deploy_python_version: explain_config_option(
                deploy_python_version.map(nrz_source_bundle::PythonMinor::version),
                if self.config.deploy.python_version.is_some()
                    || self.config.build.python_version.is_some()
                {
                    "onreza.toml"
                } else {
                    "default"
                },
                "default",
            ),
            deploy_app: explain_origin_value(self.deploy_app(), self.deploy_app_source, "absent"),
        })
    }
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
