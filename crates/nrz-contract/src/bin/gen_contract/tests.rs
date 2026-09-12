use super::normalize_for_typify;

#[test]
fn typify_normalization_rewrites_draft_2020_tuples() {
    let mut schema = serde_json::json!({
        "type": "array",
        "prefixItems": [{ "type": "string" }, { "type": "integer" }],
        "items": false,
        "minItems": 2,
        "maxItems": 2
    });

    normalize_for_typify(&mut schema);

    assert_eq!(
        schema,
        serde_json::json!({
            "type": "array",
            "items": [{ "type": "string" }, { "type": "integer" }],
            "additionalItems": false,
            "minItems": 2,
            "maxItems": 2
        })
    );
}
