use std::path::Path;

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::{ApiClient, path_segment};
use crate::auth;
use crate::cli::functions::{
    FunctionsArgs, FunctionsCheckArgs, FunctionsCommand, FunctionsInvokeArgs, FunctionsRuntimeArgs,
    FunctionsRuntimeCommand,
};
use crate::execution_context;
use crate::functions;
use crate::functions_runtime::{CachedRuntime, RuntimePreflight, RuntimeResolver, preflight};
use crate::output::{self, Phase};
use nrz::config;
use nrz::config::ProjectConfig;

const MAX_TEST_INVOKE_HEADER_COUNT: usize = 64;
const MAX_TEST_INVOKE_HEADER_NAME_LENGTH: usize = 128;
const MAX_TEST_INVOKE_HEADER_VALUE_LENGTH: usize = 8_192;
const MAX_TEST_INVOKE_BODY_BYTES: usize = 1_048_576;
const DEFAULT_TEST_INVOKE_PATH: &str = "/";
const DEFAULT_TEST_INVOKE_HOST: &str = "test-invoke.onreza.internal";

pub async fn run(
    args: FunctionsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match args.command {
        FunctionsCommand::Check(args) => check(args, json).await,
        FunctionsCommand::Invoke(args) => invoke(*args, json, token, workspace, config).await,
        FunctionsCommand::Runtime(args) => runtime(args, json).await,
    }
}

async fn check(args: FunctionsCheckArgs, json: bool) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let edge_rules = functions::check_edge_rules(&project_dir)?;
    let collected: functions::CollectedFunctions = functions::collect(&project_dir)?;
    if collected.is_empty() && edge_rules.is_none() {
        return Err(output::coded_error(
            "ONREZA_FUNCTIONS_NOT_FOUND",
            "no ONREZA Function entry files or onreza.rules.toml found",
        ));
    }

    let runtime = if collected.is_empty() {
        None
    } else {
        Some(preflight(&project_dir, &collected).await?)
    };
    let functions = collected
        .functions
        .iter()
        .map(|function| FunctionCheckItem {
            name: function.name.clone(),
            entrypoint: function.entrypoint.clone(),
        })
        .collect();

    if !json {
        report_edge_rules_human(edge_rules.as_ref());
    }

    let edge_rule_count = edge_rules.as_ref().map_or(0, |report| report.rule_count);
    if json {
        output::json_output(&FunctionCheckReport {
            runtime: runtime.clone(),
            functions,
            edge_rules: edge_rules.clone(),
        });
    }

    if !json {
        if let Some(runtime) = runtime {
            output::status(
                false,
                "✓",
                format!(
                    "{} loaded {} function(s) for {}",
                    runtime.runtime_release_id, runtime.functions_loaded, runtime.target
                ),
                Phase::Functions,
            );
        }
        output::success(
            false,
            format!(
                "check passed ({} function(s), {} edge rule(s) scanned)",
                collected.functions.len(),
                edge_rule_count
            ),
            Phase::Functions,
        );
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionCheckReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<RuntimePreflight>,
    functions: Vec<FunctionCheckItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_rules: Option<functions::EdgeRulesCheckReport>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionCheckItem {
    name: String,
    entrypoint: String,
}

async fn runtime(args: FunctionsRuntimeArgs, json: bool) -> anyhow::Result<()> {
    let resolver = RuntimeResolver::pinned()?;
    match args.command {
        FunctionsRuntimeCommand::Install => {
            let runtime = resolver.resolve().await?;
            report_installed_runtime(runtime, json);
        }
        FunctionsRuntimeCommand::Status => {
            let status = resolver.status().await?;
            if json {
                output::json_output(&status);
            } else {
                output::status(
                    false,
                    if status.installed { "✓" } else { "○" },
                    format!(
                        "{} for {} is {} at {}",
                        status.runtime_release_id,
                        status.target,
                        if status.installed {
                            "installed"
                        } else {
                            "not installed"
                        },
                        status.path.display()
                    ),
                    Phase::Functions,
                );
            }
        }
        FunctionsRuntimeCommand::Path => {
            let runtime = resolver.resolve().await?;
            if json {
                output::json_output(&runtime);
            } else {
                println!("{}", runtime.path.display());
            }
        }
    }
    Ok(())
}

fn report_installed_runtime(runtime: CachedRuntime, json: bool) {
    if json {
        output::json_output(&runtime);
    } else {
        output::success(
            false,
            format!(
                "{} for {} is ready at {}",
                runtime.runtime_release_id,
                runtime.target,
                runtime.path.display()
            ),
            Phase::Functions,
        );
    }
}

fn report_edge_rules_human(report: Option<&functions::EdgeRulesCheckReport>) {
    let Some(report) = report else {
        return;
    };
    output::status(
        false,
        "✓",
        format!(
            "{} edge rule(s) validated from {}",
            report.rule_count, report.path
        ),
        Phase::Functions,
    );
}

async fn invoke(
    args: FunctionsInvokeArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
    let environment_id = execution_context::resolve_for_mutation(
        &client,
        &project_id,
        &project_dir,
        args.environment.as_deref(),
        None,
    )
    .await?
    .environment_id;
    let functions = list_remote_functions(&client, &project_id, &environment_id).await?;
    let function = resolve_function_by_name(&functions.functions, &args.name)?;
    let revision_id = function.active_revision_id()?;
    let request = build_test_invoke_request(&args)?;
    let result_value: Value = client
        .post(
            &format!(
                "/v1/projects/{}/function-activations/environments/{}/functions/{}/revisions/{}/test-invoke",
                path_segment(&project_id),
                path_segment(&environment_id),
                path_segment(&function.id),
                path_segment(&revision_id)
            ),
            &request,
        )
        .await
        .context("failed to invoke ONREZA Function")?;
    serde_json::from_value::<nrz_contract::CliFunctionTestInvokeResponse>(result_value.clone())
        .context("ONREZA Function test invoke response does not match CLI contract")?;
    let result: FunctionTestInvokeResult = serde_json::from_value(result_value)
        .context("failed to parse ONREZA Function test invoke response")?;
    let output = FunctionInvokeOutput {
        function_name: function.name.clone(),
        function_id: function.id.clone(),
        environment_id,
        revision_id,
        result,
    };

    if json {
        output::json_output(&output);
    } else {
        report_invoke_human(&output);
    }
    Ok(())
}

async fn list_remote_functions(
    client: &ApiClient,
    project_id: &str,
    environment_id: &str,
) -> anyhow::Result<RemoteFunctionsResponse> {
    client
        .get(&format!(
            "/v1/projects/{}/function-activations/environments/{}/functions",
            path_segment(project_id),
            path_segment(environment_id)
        ))
        .await
        .context("failed to list ONREZA Functions")
}

fn resolve_function_by_name<'a>(
    functions: &'a [RemoteFunction],
    name: &str,
) -> anyhow::Result<&'a RemoteFunction> {
    let matches: Vec<&RemoteFunction> = functions
        .iter()
        .filter(|function| function.name == name)
        .collect();
    match matches.as_slice() {
        [function] => Ok(*function),
        [] => Err(output::coded_error(
            "ONREZA_FUNCTION_NOT_FOUND",
            format!("function '{name}' is not active in this environment"),
        )),
        _ => Err(output::coded_error(
            "ONREZA_FUNCTION_AMBIGUOUS",
            format!("function name '{name}' matched multiple functions"),
        )),
    }
}

pub(crate) fn build_test_invoke_request(args: &FunctionsInvokeArgs) -> anyhow::Result<Value> {
    ensure_single_stdin(&[
        args.payload.as_deref(),
        args.body.as_deref(),
        args.event.as_deref(),
        args.debug.as_deref(),
    ])?;
    reject_fetch_flags_with_event(args)?;

    let path = args.path.as_deref().unwrap_or(DEFAULT_TEST_INVOKE_PATH);
    if !path.starts_with('/') {
        return Err(output::coded_error(
            "INVALID_PATH",
            "function invoke path must start with /",
        ));
    }
    let host = args.host.as_deref().unwrap_or(DEFAULT_TEST_INVOKE_HOST);

    let mut headers = parse_headers(&args.headers)?;
    let mut method = args.method.clone();
    let mut body_base64 = match args.body_base64.as_deref() {
        Some(body_base64) => {
            validate_body_base64(body_base64)?;
            Some(body_base64.to_string())
        }
        None => None,
    };

    if let Some(payload) = args.payload.as_deref() {
        let bytes = read_json_payload_bytes(payload)?;
        validate_body_bytes_len(bytes.len())?;
        body_base64 = Some(base64::engine::general_purpose::STANDARD.encode(bytes));
        method.get_or_insert_with(|| "POST".to_string());
        if !has_header(&headers, "content-type") {
            headers.push(["content-type".to_string(), "application/json".to_string()]);
        }
    } else if let Some(body) = args.body.as_deref() {
        let bytes = read_body_bytes(body)?;
        validate_body_bytes_len(bytes.len())?;
        body_base64 = Some(base64::engine::general_purpose::STANDARD.encode(bytes));
        method.get_or_insert_with(|| "POST".to_string());
    } else if args.body_base64.is_some() {
        method.get_or_insert_with(|| "POST".to_string());
    }

    let mut request = json!({
        "method": method.unwrap_or_else(|| "GET".to_string()),
        "path": path,
        "host": host,
        "headers": headers,
    });
    let object = request
        .as_object_mut()
        .expect("test invoke request must be a JSON object");
    if let Some(query_string) = &args.query_string {
        object.insert(
            "queryString".to_string(),
            Value::String(query_string.clone()),
        );
    }
    if let Some(body_base64) = body_base64 {
        object.insert("bodyBase64".to_string(), Value::String(body_base64));
    }
    if let Some(event_path) = args.event.as_deref() {
        object.insert("event".to_string(), read_json_value(event_path, "event")?);
    }
    if let Some(debug_path) = args.debug.as_deref() {
        object.insert(
            "debug".to_string(),
            read_json_value(debug_path, "debug options")?,
        );
    }

    serde_json::from_value::<nrz_contract::CliFunctionTestInvokeRequest>(request.clone())
        .context("ONREZA Function test invoke request does not match CLI contract")?;
    Ok(request)
}

fn reject_fetch_flags_with_event(args: &FunctionsInvokeArgs) -> anyhow::Result<()> {
    if args.event.is_none() {
        return Ok(());
    }

    let mut flags = Vec::new();
    if args.method.is_some() {
        flags.push("--method");
    }
    if args.path.is_some() {
        flags.push("--path");
    }
    if args.query_string.is_some() {
        flags.push("--query-string");
    }
    if args.host.is_some() {
        flags.push("--host");
    }
    if !args.headers.is_empty() {
        flags.push("--header");
    }
    if args.payload.is_some() {
        flags.push("--payload");
    }
    if args.body.is_some() {
        flags.push("--body");
    }
    if args.body_base64.is_some() {
        flags.push("--body-base64");
    }
    if flags.is_empty() {
        return Ok(());
    }

    Err(output::coded_error(
        "INVALID_INVOKE_MODE",
        format!(
            "--event cannot be combined with fetch request flag(s): {}",
            flags.join(", ")
        ),
    ))
}

fn parse_headers(headers: &[String]) -> anyhow::Result<Vec<[String; 2]>> {
    if headers.len() > MAX_TEST_INVOKE_HEADER_COUNT {
        return Err(output::coded_error(
            "INVALID_HEADER",
            format!("too many headers; max {MAX_TEST_INVOKE_HEADER_COUNT}"),
        ));
    }

    headers
        .iter()
        .map(|header| {
            let Some((name, value)) = header.split_once(':') else {
                return Err(output::coded_error(
                    "INVALID_HEADER",
                    format!("invalid header '{header}'; expected 'Name: value'"),
                ));
            };
            let name = name.trim();
            if name.is_empty() {
                return Err(output::coded_error(
                    "INVALID_HEADER",
                    format!("invalid header '{header}'; header name must not be empty"),
                ));
            }
            if name.chars().count() > MAX_TEST_INVOKE_HEADER_NAME_LENGTH {
                return Err(output::coded_error(
                    "INVALID_HEADER",
                    format!(
                        "invalid header '{header}'; header name must be at most {MAX_TEST_INVOKE_HEADER_NAME_LENGTH} characters"
                    ),
                ));
            }
            let value = value.trim_start();
            if value.chars().count() > MAX_TEST_INVOKE_HEADER_VALUE_LENGTH {
                return Err(output::coded_error(
                    "INVALID_HEADER",
                    format!(
                        "invalid header '{header}'; header value must be at most {MAX_TEST_INVOKE_HEADER_VALUE_LENGTH} characters"
                    ),
                ));
            }
            Ok([name.to_string(), value.to_string()])
        })
        .collect()
}

fn validate_body_base64(value: &str) -> anyhow::Result<()> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| {
            output::coded_error(
                "INVALID_BODY",
                "bodyBase64 must be standard base64 and at most 1 MiB decoded",
            )
        })?;
    validate_body_bytes_len(bytes.len()).map_err(|_| {
        output::coded_error(
            "INVALID_BODY",
            "bodyBase64 must be standard base64 and at most 1 MiB decoded",
        )
    })
}

fn validate_body_bytes_len(len: usize) -> anyhow::Result<()> {
    if len > MAX_TEST_INVOKE_BODY_BYTES {
        return Err(output::coded_error(
            "INVALID_BODY",
            "request body must be at most 1 MiB",
        ));
    }
    Ok(())
}

fn has_header(headers: &[[String; 2]], name: &str) -> bool {
    headers
        .iter()
        .any(|[header_name, _]| header_name.eq_ignore_ascii_case(name))
}

fn ensure_single_stdin(paths: &[Option<&str>]) -> anyhow::Result<()> {
    let count = paths
        .iter()
        .filter(|path| path.is_some_and(|path| path == "-"))
        .count();
    if count > 1 {
        return Err(output::coded_error(
            "INVALID_STDIN_USAGE",
            "stdin can only be used by one of --payload, --body, --event, or --debug",
        ));
    }
    Ok(())
}

fn read_json_payload_bytes(path: &str) -> anyhow::Result<Vec<u8>> {
    let value = read_json_value(path, "payload")?;
    serde_json::to_vec(&value).context("failed to serialize JSON payload")
}

fn read_json_value(path: &str, label: &str) -> anyhow::Result<Value> {
    let content = if path == "-" {
        let mut content = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut content)
            .with_context(|| format!("failed to read {label} from stdin"))?;
        content
    } else {
        std::fs::read_to_string(path).with_context(|| format!("failed to read {label} {path}"))?
    };
    serde_json::from_str(&content).with_context(|| format!("failed to parse {label} as JSON"))
}

fn read_body_bytes(path: &str) -> anyhow::Result<Vec<u8>> {
    if path == "-" {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes)
            .context("failed to read request body from stdin")?;
        Ok(bytes)
    } else {
        std::fs::read(path).with_context(|| format!("failed to read request body {path}"))
    }
}

fn report_invoke_human(output: &FunctionInvokeOutput) {
    let invocation = &output.result.invocation;
    let status = invocation
        .response
        .as_ref()
        .and_then(|response| response.status)
        .map_or_else(|| "-".to_string(), |status| status.to_string());
    let duration = invocation
        .timings
        .as_ref()
        .and_then(|timings| timings.total_ms)
        .map_or_else(|| "-".to_string(), |value| format!("{value:.3} ms"));
    output::success(
        false,
        format!(
            "invoked {} (status {status}, duration {duration})",
            output.function_name
        ),
        Phase::Functions,
    );

    if let Some(body) = invocation.response.as_ref().and_then(render_response_body) {
        eprintln!();
        eprintln!("Body:");
        eprintln!("{}", output::terminal_text(&body));
    }
    if !invocation.logs.is_empty() {
        eprintln!();
        eprintln!("Logs:");
        for log in &invocation.logs {
            eprintln!("  {}", output::terminal_text(&render_log_line(log)));
        }
    }
    if !invocation.ok
        && let Some(error) = &invocation.error
    {
        eprintln!();
        eprintln!("Error:");
        eprintln!("{}", output::terminal_text(&error.to_string()));
    }
}

pub(crate) fn render_response_body(response: &FunctionInvokeResponse) -> Option<String> {
    if let Some(body_base64) = response
        .body_base64
        .as_deref()
        .filter(|body| !body.is_empty())
    {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(body_base64)
            .ok()?;
        return match String::from_utf8(bytes) {
            Ok(text) => Some(text),
            Err(error) => Some(format!("<{} bytes binary body>", error.into_bytes().len())),
        };
    }
    response.body_preview.clone()
}

fn render_log_line(log: &Value) -> String {
    let Some(object) = log.as_object() else {
        return log.to_string();
    };
    let level = object.get("level").and_then(Value::as_str).unwrap_or("log");
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| log.to_string());
    if let Some(properties) = object.get("properties").filter(|value| !value.is_null()) {
        format!("[{level}] {message} {properties}")
    } else {
        format!("[{level}] {message}")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFunctionsResponse {
    functions: Vec<RemoteFunction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFunction {
    id: String,
    name: String,
    activation: Option<RemoteFunctionActivation>,
}

impl RemoteFunction {
    fn active_revision_id(&self) -> anyhow::Result<String> {
        self.activation
            .as_ref()
            .and_then(|activation| activation.active_revision_id.clone())
            .ok_or_else(|| {
                output::coded_error(
                    "ONREZA_FUNCTION_REVISION_NOT_FOUND",
                    format!("function '{}' has no active revision", self.name),
                )
            })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFunctionActivation {
    active_revision_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionInvokeOutput {
    function_name: String,
    function_id: String,
    environment_id: String,
    revision_id: String,
    result: FunctionTestInvokeResult,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionTestInvokeResult {
    invocation: FunctionInvocation,
    debug_trace: Value,
    revision: FunctionInvokedRevision,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionInvokedRevision {
    id: String,
    function_id: String,
    source_snapshot_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionInvocation {
    invocation_id: String,
    ok: bool,
    timings: Option<FunctionInvocationTimings>,
    response: Option<FunctionInvokeResponse>,
    error: Option<Value>,
    #[serde(default)]
    logs: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionInvocationTimings {
    total_ms: Option<f64>,
    worker_ms: Option<f64>,
    wait_until_ms: Option<f64>,
    cold_worker_start_ms: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FunctionInvokeResponse {
    pub(crate) status: Option<u16>,
    #[serde(default)]
    pub(crate) headers: Vec<[String; 2]>,
    pub(crate) body_base64: Option<String>,
    pub(crate) body_preview: Option<String>,
}
