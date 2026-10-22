use super::*;
use models_soup::agent_session::{AgentPullRequestState, SoupAgentSession};

fn agent(id: Uuid, pull_request: Option<&str>) -> SoupCandidate {
    SoupCandidate::plain(SoupItem::AgentSession(SoupAgentSession {
        id,
        name: "Agent".to_owned(),
        is_archived: false,
        owner_id: Owner::User(MacroUserIdStr::parse_from_str("macro|owner@example.com").unwrap()),
        bot_id: Uuid::now_v7(),
        harness: "macrod".to_owned(),
        repo_url: Some("https://github.com/macro/macro".to_owned()),
        repo_branch: Some("main".to_owned()),
        pull_request_url: pull_request.map(str::to_owned),
        working_branch: None,
        pull_request_state: None,
        pull_request_id: None,
        turn_state: None,
        thread_id: None,
        status: "session/end".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        viewed_at: None,
        extra: (),
    }))
}

fn pull_request(id: &str, user: &str, status: &str) -> ForeignEntity {
    ForeignEntity {
        id: Uuid::now_v7(),
        foreign_entity_id: format!("macro/macro/pull/{id}"),
        foreign_entity_source: "github_pull_request".to_owned(),
        metadata: serde_json::json!({ "status": status }),
        stored_for_id: user.to_owned(),
        stored_for_auth_entity: "user".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn metadata_is_batched_and_only_uses_visible_prs() {
    let user = "macro|owner@example.com";
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let third = Uuid::now_v7();
    let foreign = RecordingPullRequestListing::new(vec![
        pull_request("1", user, "merged"),
        pull_request("2", "macro|someone-else@example.com", "closed"),
        pull_request("3", user, "unknown"),
    ]);
    let mut items = vec![
        agent(first, Some("https://github.com/macro/macro/pull/1/files")),
        agent(second, Some("https://github.com/macro/macro/pull/2")),
        agent(third, Some("https://github.com/macro/macro/pull/3")),
    ];
    super::super::agent_metadata::enrich(
        &foreign,
        user.to_owned(),
        vec![SourceId::user(user)],
        &mut items,
    )
    .await
    .unwrap();

    let sessions: Vec<_> = items
        .iter()
        .map(|item| match &item.item {
            SoupItem::AgentSession(session) => session,
            _ => unreachable!(),
        })
        .collect();

    assert_eq!(
        sessions[0].pull_request_state,
        Some(AgentPullRequestState::Merged)
    );
    assert_eq!(sessions[1].working_branch, None);
    assert_eq!(
        sessions[1].pull_request_state, None,
        "another user's PR is hidden"
    );
    assert_eq!(sessions[1].pull_request_id, None);
    assert!(sessions[0].pull_request_id.is_some());
    assert!(sessions[2].pull_request_id.is_some());
    assert_eq!(sessions[2].pull_request_state, None, "unknown is not open");
    let calls = foreign.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].limit, 3);
    assert_eq!(calls[0].source_ids, vec![SourceId::user(user)]);
}

#[tokio::test]
async fn no_pr_links_skip_foreign_entity_lookup() {
    let foreign = RecordingPullRequestListing::new(Vec::new());
    let mut items = vec![agent(Uuid::now_v7(), None)];
    super::super::agent_metadata::enrich(
        &foreign,
        "macro|owner@example.com".to_owned(),
        vec![SourceId::user("macro|owner@example.com")],
        &mut items,
    )
    .await
    .unwrap();
    assert!(foreign.calls().is_empty());
}

#[tokio::test]
async fn runtime_branch_without_pr_survives_enrichment_and_takes_precedence() {
    let session = Uuid::now_v7();
    let foreign = RecordingPullRequestListing::new(Vec::new());
    let mut item = agent(session, None);
    let SoupItem::AgentSession(row) = &mut item.item else {
        unreachable!()
    };
    row.working_branch = Some("cursor/no-pr".to_owned());
    let mut items = vec![item];
    super::super::agent_metadata::enrich(
        &foreign,
        "macro|owner@example.com".to_owned(),
        vec![SourceId::user("macro|owner@example.com")],
        &mut items,
    )
    .await
    .unwrap();
    let SoupItem::AgentSession(row) = &items[0].item else {
        unreachable!()
    };
    assert_eq!(row.working_branch.as_deref(), Some("cursor/no-pr"));
    assert_eq!(row.pull_request_url, None);
    assert!(foreign.calls().is_empty());
}
