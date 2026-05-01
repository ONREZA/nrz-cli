use super::*;

#[test]
fn create_body_serializes_server_contract() {
    let body = CreateBody {
        db_name: Some("primary".to_string()),
        cu_size: Some(0.5),
    };

    let value = serde_json::to_value(body).expect("create body serializes");

    assert_eq!(
        value,
        serde_json::json!({
            "dbName": "primary",
            "cuSize": 0.5,
        })
    );
    assert!(value.get("name").is_none());
}

#[test]
fn branch_list_deserializes_current_server_array() {
    let list: BranchListResponse = serde_json::from_value(serde_json::json!([
        {"id": "branch-1", "name": "main", "status": "ACTIVE"}
    ]))
    .expect("server branch array deserializes");

    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "branch-1");
    assert_eq!(list.data[0].name, "main");
}

#[test]
fn branch_list_deserializes_legacy_data_envelope() {
    let list: BranchListResponse = serde_json::from_value(serde_json::json!({
        "data": [
            {"id": "branch-2", "name": "preview", "isPreviewBranch": true}
        ]
    }))
    .expect("legacy branch envelope deserializes");

    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "branch-2");
    assert_eq!(list.data[0].is_preview_branch, Some(true));
}
