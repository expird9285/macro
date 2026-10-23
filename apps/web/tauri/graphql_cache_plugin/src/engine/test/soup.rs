use super::*;
use serde_json::{Value, json};

const VIEWER: &str = "macro|native@example.com";
const TIMESTAMP: &str = "2025-01-04T00:00:00Z";

fn id(n: u16) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

fn filters() -> Value {
    let nil = id(0);
    json!({
        "documentFilter": {"literal": {"id": nil}},
        "projectFilter": {"literal": {"projectId": nil}},
        "chatFilter": {"literal": {"chatId": nil}},
        "channelFilter": {"literal": {"channelId": nil}},
        "channelThreadFilter": {"literal": {"channelId": nil}},
        "agentSessionFilter": {"literal": {"id": nil}},
        "callFilter": {"literal": {"callId": nil}},
        "foreignEntityFilter": {"literal": {"id": nil}},
    })
}

fn request(limit: u16) -> EntityFilterRequest {
    serde_json::from_value(json!({"filters": filters(), "sortMethod": "UPDATED_AT",
        "sortDirection": "DESC", "limit": limit}))
    .unwrap()
}

fn evaluate(handle: &EngineHandle, request: EntityFilterRequest) -> Value {
    serde_json::to_value(block_on(handle.entity_filter(request)).unwrap()).unwrap()
}

fn write_data(
    handle: &EngineHandle,
    query: &str,
    data: Value,
    identity: Option<&str>,
) -> WriteResultWire {
    block_on(handle.write(WriteRequest {
        origin_op_id: None,
        registration: None,
        query: query.into(),
        operation_name: None,
        variables: Variables::new(),
        data,
        identity: identity.map(str::to_owned),
    }))
    .unwrap()
}

#[test]
fn native_live_query_returns_field_deltas_from_the_shared_engine() {
    let handle = spawn_handle();
    let project_key = format!("GraphqlSoupProject:{}", id(100));
    let project_data = |favorite| {
        json!({"user": {"id": VIEWER, "soup": {
            "items": [{"__typename": "GraphqlSoupProject", "id": id(100), "cacheProjection": null,
                "ownerId": VIEWER, "parentId": null, "createdAt": TIMESTAMP, "updatedAt": TIMESTAMP,
                "notifications": [], "properties": [], "isFavorited": favorite}],
        }}})
    };
    write_data(
        &handle,
        include_str!("project.graphql"),
        project_data(false),
        Some(VIEWER),
    );
    let mut filters = filters();
    filters["projectFilter"] = json!({"literal": {"projectIdSelf": id(100)}});
    let live_request = |since: Option<&str>| {
        serde_json::from_value(json!({
        "filters": filters, "sortMethod": "UPDATED_AT", "sortDirection": "DESC", "limit": 20,
        "baseline": [{"key":project_key, "sortTimestamp":TIMESTAMP}],
        "liveQuery": { "id":"native-live", "document":"fragment Item on GraphqlSoupProject { id favorite: isFavorited }", "fragmentName":"Item", "since":since },
    })).unwrap()
    };
    let first = evaluate(&handle, live_request(None));
    assert_eq!(first["kind"], "live-query");
    assert_eq!(first["upserts"].as_array().unwrap().len(), 1);
    write_data(
        &handle,
        include_str!("project.graphql"),
        project_data(true),
        Some(VIEWER),
    );
    let next = evaluate(&handle, live_request(first["revision"].as_str()));
    assert_eq!(next["reset"], false);
    assert!(next.get("keys").is_none());
    assert!(next["upserts"].as_array().unwrap().is_empty());
    assert_eq!(next["patches"].as_array().unwrap().len(), 1);
    assert_eq!(
        next["patches"][0]["fields"],
        json!([{ "path":["favorite"], "value":true }])
    );
}

#[test]
fn native_general_soup_filter_and_server_baseline_reconciliation() {
    let handle = spawn_handle();
    let project_key = format!("GraphqlSoupProject:{}", id(100));
    write_data(
        &handle,
        include_str!("project.graphql"),
        json!({"user": {"id": VIEWER, "soup": {
            "items": [{"__typename": "GraphqlSoupProject", "id": id(100), "cacheProjection": null,
                "ownerId": VIEWER, "parentId": null, "createdAt": TIMESTAMP, "updatedAt": TIMESTAMP,
                "notifications": [], "properties": [], "isFavorited": false}],
        }}}),
        Some(VIEWER),
    );
    let general_request = || {
        let mut request = request(100);
        request.filters["projectFilter"] = json!({"literal": {"projectIdSelf": id(100)}});
        request
    };
    let complete = evaluate(&handle, general_request());
    assert_eq!(complete["kind"], "complete");
    assert_eq!(complete["keys"], json!([project_key]));
    block_on(handle.invalidate(vec![project_key.clone()])).unwrap();
    let mut reconciled = general_request();
    reconciled.baseline = Some(vec![PredicateBaselineEntry {
        key: project_key.clone(),
        sort_timestamp: TIMESTAMP.into(),
    }]);
    let page = evaluate(&handle, reconciled);
    assert_eq!(page["kind"], "reconciled");
    assert_eq!(page["keys"], json!([project_key]));
    assert_eq!(page["retainedKeys"], json!([project_key]));
    let mut too_many = general_request();
    too_many.baseline = Some(
        (0..5001)
            .map(|_| PredicateBaselineEntry {
                key: project_key.clone(),
                sort_timestamp: TIMESTAMP.into(),
            })
            .collect(),
    );
    let error = block_on(handle.entity_filter(too_many)).err().unwrap();
    assert_eq!(error, "reconciliation baseline is too large");
    let mut unsupported = general_request();
    unsupported.sort_method = "VIEWED_UPDATED".into();
    assert_eq!(evaluate(&handle, unsupported)["kind"], "unsupported");
}

#[test]
fn native_filter_rejects_malformed_requests_and_preserves_unsupported() {
    let handle = spawn_handle();
    assert!(block_on(handle.entity_filter(request(0))).is_err());
    let mut unsupported = request(100);
    unsupported.sort_method = "VIEWED_UPDATED".into();
    assert_eq!(evaluate(&handle, unsupported)["kind"], "unsupported");
    assert!(
        serde_json::from_value::<EntityFilterRequest>(json!({
            "filters": {}, "sortMethod": "UPDATED_AT", "sortDirection": "DESC", "limit": -1,
        }))
        .is_err()
    );
}
