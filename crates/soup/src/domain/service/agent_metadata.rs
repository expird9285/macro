//! Persisted coding context for the already-authorized agent rows in one page.

use std::collections::{HashMap, HashSet};

use github_pull_requests::domain::models::diff::PullRequestRef;
use models_soup::agent_session::AgentPullRequestState;

use super::*;

fn pull_request_key(url: &str) -> Option<String> {
    let reference = PullRequestRef::parse(url)?;
    Some(format!(
        "{}/pull/{}",
        reference.repository, reference.number
    ))
}

/// Enrich only returned rows, so metadata work is bounded by the page size and
/// never reads branch facts for sessions outside the caller's visible page.
pub(super) async fn enrich<F: GithubPullRequestListing>(
    pull_requests: &F,
    user: String,
    sources: Vec<SourceId>,
    items: &mut [SoupCandidate],
) -> Result<(), SoupErr> {
    if !items
        .iter()
        .any(|candidate| matches!(candidate.item, SoupItem::AgentSession(_)))
    {
        return Ok(());
    }

    let keys: HashSet<_> = items
        .iter()
        .filter_map(|candidate| match &candidate.item {
            SoupItem::AgentSession(session) => session
                .pull_request_url
                .as_deref()
                .and_then(pull_request_key),
            _ => None,
        })
        .collect();

    let pr_facts = async {
        let Some(ids) = balanced_or_tree(
            keys.iter()
                .cloned()
                .map(ForeignEntityLiteral::ForeignEntityId)
                .map(Expr::Literal)
                .collect(),
        ) else {
            return Ok(Vec::new());
        };
        pull_requests
            .list_pull_requests(
                Some(user),
                sources,
                keys.len() as u32,
                Query::new(None, SimpleSortMethod::UpdatedAt, Some(ids)),
                None,
                GithubPullRequestSortDirection::Desc,
            )
            .await
            .map_err(anyhow::Error::from)
    };
    let pr_facts = pr_facts.await?;
    let states: HashMap<_, (Uuid, Option<AgentPullRequestState>)> = pr_facts
        .into_iter()
        .map(|entity| {
            let state = entity
                .metadata
                .get("status")
                .and_then(|status| serde_json::from_value(status.clone()).ok());
            (entity.foreign_entity_id, (entity.id, state))
        })
        .collect();

    for candidate in items {
        let SoupItem::AgentSession(session) = &mut candidate.item else {
            continue;
        };
        let pull_request = session
            .pull_request_url
            .as_deref()
            .and_then(pull_request_key)
            .and_then(|key| states.get(&key).copied());
        session.pull_request_id = pull_request.map(|(id, _)| id);
        session.pull_request_state = pull_request.and_then(|(_, state)| state);
    }
    Ok(())
}
