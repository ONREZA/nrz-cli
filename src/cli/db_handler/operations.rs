use super::*;
pub(super) use nrz_api::Database200Response2ObjectOperationOperationType as OperationType;
use nrz_api::Database200ResponseDatumRuntimeOperationStatus as Status;
use serde::Serialize;

/// Allowlisted CLI projection: generated wire values can contain credentials and extra properties.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DatabaseOperation {
    operation_id: uuid::Uuid,
    operation_type: OperationType,
    status: Status,
    result: OperationResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<uuid::Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResult {
    database_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_lsn: Option<String>,
}

impl From<nrz_api::Database200Response2ObjectOperation> for DatabaseOperation {
    fn from(value: nrz_api::Database200Response2ObjectOperation) -> Self {
        Self {
            operation_id: value.operation_id,
            operation_type: value.operation_type,
            status: value.status,
            result: OperationResult {
                database_id: value.result.database_id,
                branch_id: value.result.branch_id,
                endpoint_id: value.result.endpoint_id,
                resolved_lsn: value.result.resolved_lsn,
            },
            error_code: value.error_code,
            pending_reason: value.pending_reason,
            idempotency_key: None,
        }
    }
}

impl DatabaseOperation {
    pub(super) fn created(value: nrz_api::Database200Response2Object2, key: uuid::Uuid) -> Self {
        // The POST-only password is deliberately never part of the CLI projection.
        let mut operation = Self::from(nrz_api::Database200Response2ObjectOperation {
            operation_id: value.operation_id,
            operation_type: value.operation_type,
            status: value.status,
            result: value.result,
            error_code: value.error_code,
            pending_reason: value.pending_reason,
            ..Default::default()
        });
        operation.idempotency_key = Some(key);
        operation
    }

    pub(super) fn with_key(mut self, key: uuid::Uuid) -> Self {
        self.idempotency_key = Some(key);
        self
    }
    pub(super) fn database_id(&self) -> String {
        self.result.database_id.to_string()
    }
    fn pending(&self) -> bool {
        matches!(self.status, Status::Pending | Status::Running)
    }
    pub(super) fn report_pending_create(&self, json: bool) {
        if !self.pending() {
            return;
        }
        let id = self.operation_id;
        let database = self.result.database_id;
        let message = format!(
            "Database creation accepted. Resume: nrz db operation {id} --database {database} --wait"
        );
        if json {
            output::log_info_structured(
                output::Phase::Db.as_str(),
                &message,
                &serde_json::json!({"operationId": id, "databaseId": database}),
            );
        } else {
            output::status(false, "~", message, output::Phase::Db);
        }
    }
    pub(super) fn check_outcome(&self) -> anyhow::Result<()> {
        if matches!(self.status, Status::Failed | Status::Cancelled) {
            bail!(
                "database operation {} {}: {}",
                self.operation_id,
                self.status,
                output::terminal_line(
                    self.error_code
                        .as_deref()
                        .unwrap_or("operation did not complete")
                )
            );
        }
        Ok(())
    }
    pub(super) fn verify_type(&self, expected: OperationType) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.operation_type == expected,
            "operation response has another command type"
        );
        Ok(())
    }
    pub(super) fn verify_branch(&self, expected: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.result.branch_id.as_deref() == Some(expected),
            "operation belongs to another branch"
        );
        Ok(())
    }
    pub(super) fn verify_database(&self, database: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.result.database_id == database.parse::<uuid::Uuid>()?,
            "operation belongs to another database"
        );
        Ok(())
    }
}

pub(super) fn print_operation(operation: &DatabaseOperation, json: bool) {
    if json {
        output::json_output(operation);
        return;
    }
    eprintln!(
        "  Operation {}: {} ({})",
        operation.operation_id, operation.status, operation.operation_type
    );
    eprintln!("  Database: {}", operation.result.database_id);
    if let Some(branch) = &operation.result.branch_id {
        eprintln!("  Branch: {}", output::terminal_line(branch));
    }
    if let Some(reason) = operation
        .pending_reason
        .as_deref()
        .or(operation.error_code.as_deref())
    {
        eprintln!("  Reason: {}", output::terminal_line(reason));
    }
    if let Some(key) = operation.idempotency_key {
        eprintln!("  Idempotency-Key: {key}");
    }
    if operation.pending() {
        eprintln!(
            "  Check: nrz db operation {} --database {}",
            operation.operation_id, operation.result.database_id
        );
    }
}

pub(super) async fn read_operation(
    client: &ApiClient,
    database: &str,
    operation_id: uuid::Uuid,
) -> anyhow::Result<DatabaseOperation> {
    let operation = DatabaseOperation::from(
        client
            .database_operation(database, operation_id)
            .await
            .with_context(|| format!("failed to read database operation {operation_id}"))?,
    );
    operation.verify_database(database)?;
    anyhow::ensure!(
        operation.operation_id == operation_id,
        "operation response has another identity"
    );
    Ok(operation)
}

pub(super) async fn wait_operation(
    client: &ApiClient,
    mut operation: DatabaseOperation,
) -> anyhow::Result<DatabaseOperation> {
    let id = operation.operation_id;
    let database = operation.database_id();
    let result = tokio::time::timeout(std::time::Duration::from_secs(240), async {
        while operation.pending() {
            let mut next = read_operation(client, &database, id).await?;
            anyhow::ensure!(
                next.operation_type == operation.operation_type,
                "operation type changed during observation"
            );
            for (previous, current) in [
                (&operation.result.branch_id, &next.result.branch_id),
                (&operation.result.endpoint_id, &next.result.endpoint_id),
            ] {
                anyhow::ensure!(
                    previous.is_none() || previous == current,
                    "operation resource identity changed during observation"
                );
            }
            next.idempotency_key = operation.idempotency_key;
            operation = next;
            if operation.pending() {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
        operation.check_outcome()?;
        Ok::<_, anyhow::Error>(operation)
    })
    .await;
    result.with_context(|| {
        format!(
            "operation is still in progress; check: nrz db operation {id} --database {database}"
        )
    })?
}

pub(super) async fn cmd_operation(
    client: &ApiClient,
    database: &str,
    operation_id: uuid::Uuid,
    wait: bool,
    json: bool,
) -> anyhow::Result<()> {
    let mut operation = read_operation(client, database, operation_id).await?;
    if wait {
        operation = wait_operation(client, operation).await?;
    }
    print_operation(&operation, json);
    Ok(())
}
