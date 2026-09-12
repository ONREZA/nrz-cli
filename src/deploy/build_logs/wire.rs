use super::{
    ApiClient, BuildLogEvent, BuildLogLevel, BuildLogOrigin, BuildLogPhase, BuildLogSource,
    BuildLogStream,
};
use anyhow::{Context, bail};
use nrz_api::*;

pub(super) struct SessionInput<'a> {
    pub id: &'a str,
    pub project_id: &'a str,
    pub deployment_id: &'a str,
    pub attempt: u32,
    pub producer_id: &'a str,
    pub source: BuildLogSource,
    pub builder_version: Option<&'a str>,
}

pub(super) struct SessionCursor {
    pub id: String,
    pub shipping_enabled: bool,
    pub next_seq: u32,
    pub accepted_bytes: usize,
}

pub(super) async fn create_session(
    client: &ApiClient,
    input: SessionInput<'_>,
) -> anyhow::Result<SessionCursor> {
    let body = BuildLogSessionRequestBody {
        id: input.id.parse().context("invalid log session ID")?,
        project_id: input.project_id.parse().context("invalid project ID")?,
        deployment_id: input
            .deployment_id
            .parse()
            .context("invalid deployment ID")?,
        attempt: i64::from(input.attempt),
        producer_id: input
            .producer_id
            .parse()
            .context("invalid log producer ID")?,
        source: input.source.into(),
        cli_version: Some(env!("CARGO_PKG_VERSION").into()),
        builder_version: input.builder_version.map(str::to_owned),
    };
    let response = client.create_build_log_session(body.clone()).await?.session;
    if response.id != body.id
        || response.project_id != body.project_id
        || response.deployment_id != Some(body.deployment_id)
        || response.attempt != Some(body.attempt)
        || response.source != body.source
    {
        bail!("build log session response belongs to a different deployment attempt");
    }
    Ok(SessionCursor {
        id: response.id.to_string(),
        shipping_enabled: response.shipping_policy
            == BuildLogSession200ResponseSessionShippingPolicy::Enabled,
        next_seq: response
            .next_seq
            .try_into()
            .context("invalid build log cursor")?,
        accepted_bytes: response
            .accepted_bytes
            .try_into()
            .context("invalid build log byte count")?,
    })
}

impl TryFrom<&BuildLogEvent> for EventRequestBodyItem {
    type Error = anyhow::Error;
    fn try_from(event: &BuildLogEvent) -> anyhow::Result<Self> {
        Ok(Self {
            seq: i64::from(event.seq),
            timestamp: Some(event.timestamp.parse().context("invalid log timestamp")?),
            stream: event.stream.into(),
            level: event.level.into(),
            phase: event.phase.into(),
            message: event.message.clone(),
            origin: event.origin.into(),
            code: None,
            metadata: None,
        })
    }
}

impl From<BuildLogSource> for BuildLogSessionRequestBodySource {
    fn from(value: BuildLogSource) -> Self {
        match value {
            BuildLogSource::LocalCli => Self::LocalCli,
            BuildLogSource::RemoteBuilder => Self::RemoteBuilder,
        }
    }
}

impl From<BuildLogStream> for EventRequestBodyItemStream {
    fn from(value: BuildLogStream) -> Self {
        match value {
            BuildLogStream::User => Self::User,
            BuildLogStream::Debug => Self::Debug,
        }
    }
}

impl From<BuildLogLevel> for EventRequestBodyItemLevel {
    fn from(value: BuildLogLevel) -> Self {
        match value {
            BuildLogLevel::Debug => Self::Debug,
            BuildLogLevel::Info => Self::Info,
            BuildLogLevel::Warn => Self::Warn,
            BuildLogLevel::Error => Self::Error,
        }
    }
}

impl From<BuildLogPhase> for BuildLogSession200ResponseSessionFailurePhase {
    fn from(value: BuildLogPhase) -> Self {
        match value {
            BuildLogPhase::Init => Self::Init,
            BuildLogPhase::Detect => Self::Detect,
            BuildLogPhase::Install => Self::Install,
            BuildLogPhase::Build => Self::Build,
            BuildLogPhase::Deploy => Self::Deploy,
            BuildLogPhase::Upload => Self::Upload,
            BuildLogPhase::Activate => Self::Activate,
            BuildLogPhase::Complete => Self::Complete,
            BuildLogPhase::Error => Self::Error,
        }
    }
}

impl From<BuildLogOrigin> for EventRequestBodyItemOrigin {
    fn from(value: BuildLogOrigin) -> Self {
        match value {
            BuildLogOrigin::Cli => Self::Cli,
            BuildLogOrigin::ChildStdout => Self::ChildStdout,
            BuildLogOrigin::ChildStderr => Self::ChildStderr,
            BuildLogOrigin::Builder => Self::Builder,
        }
    }
}
