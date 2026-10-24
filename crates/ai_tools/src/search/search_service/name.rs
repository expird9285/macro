use super::context::SearchToolContext;
use super::types::{
    PAGE_SIZE, SearchEntityType, SearchMatchType, SearchToolResponse, annotate_result_tags,
    resolve_tag_filters, search_filters, tag_filter_mode,
};
use ai_toolset::{AsyncTool, RequestContext, ServiceContext, ToolCallError, ToolResult};
use ai_toolset::{ToolAnnotated, ToolAnnotations};
use async_trait::async_trait;
use item_filters::EntityFilters;
use models_properties::service::tag_sets::{TagFilter, TagMatch};
use models_search::channel::ChannelNameSearchRequest;
use models_search::unified::{UnifiedSearchRequest, UnifiedSearchResponseItem};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, JsonSchema, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[schemars(
    description = "Search items by their name or title: document name, agent session title, channel name, project name, or call-record name. This is keyword search, not semantic search: queries only match literal words/tokens, prefixes, or exact quoted terms that appear in the indexed title/name. Use this for targeted name/title lookup, not for activity-summary questions like \"what happened today\", \"what's going on\", \"catch me up\", or \"what happened in standup today\"; those should start with ListEntities using time/type/channel filters. The whole query is matched as a single adjacent phrase prefix — so pass 1-3 targeted keywords drawn from words that would literally appear in the title, not the user's natural-language description; long phrases will not match. Matching defaults to prefix; set matchType to 'exact' to match whole tokens/phrases with no prefix expansion. Wrap a multi-word phrase in double quotes to keep it together as one adjacent phrase. If the user's request combines a person with a topic, run separate searches (NameSearch for the person, ContentSearch for the topic) rather than one combined query. Leave entityTypes empty by default; only filter when the user explicitly scopes to a type. Results for documents, projects, and call records include the tags visible to the user as {label, scope} pairs; to restrict a search to tagged items, pass the tag labels in the tags argument (ListTags shows which tags exist).",
    title = "NameSearch"
)]
pub struct NameSearch {
    #[schemars(
        description = "The name or title to search. Pass 1-3 keywords drawn from words that would literally appear in the title, not the user's natural-language description. The whole query is matched as a single adjacent phrase prefix, so long phrases will not match. Wrap a multi-word phrase in double quotes to match it as one phrase. Use matchType 'exact' for whole-token matching instead of prefix."
    )]
    pub name: String,

    #[schemars(
        description = "Matching mode. 'partial' (the default) matches each single word as a prefix, so `scri` matches `script`. 'exact' matches each term as a complete token or phrase with no prefix expansion — use it when the user wants an exact word, identifier, code, or quoted phrase rather than a prefix."
    )]
    #[serde(default)]
    pub match_type: SearchMatchType,

    #[schemars(
        description = "Which types of items to search. Leave empty (the default) to search all types — this is almost always what you want. Only set this when the user's request clearly targets one or more specific types. Examples: ['documents'], ['channels'], ['call_records']."
    )]
    #[serde(default)]
    pub entity_types: Vec<SearchEntityType>,

    #[schemars(description = "\
Restrict results to items carrying the given tags — any of them by default, every one of \
them with tagsMatch=\"all\". Each entry names a tag by its label, matched case-insensitively \
against the user's own tags; only set scope (\"personal\" or \"team\") when the user \
distinguishes between their personal and team tags. An unknown label fails with the list of \
available tags — call ListTags first when unsure what tags exist. Only taggable items \
(documents, projects, call records) can match, so \
channels are dropped \
while a tag filter is active.")]
    #[serde(default)]
    pub tags: Option<Vec<TagFilter>>,

    #[schemars(description = "\
How multiple entries in tags combine: \"any\" (the default) matches items carrying at least \
one of the tags, \"all\" only items carrying every one of them. With \"all\", a label that \
exists in both the personal and team sets is ambiguous — set scope on that entry to pick one. \
Ignored unless tags is set.")]
    #[serde(default)]
    pub tags_match: TagMatch,
}

impl ToolAnnotated for NameSearch {
    const ANNOTATIONS: ToolAnnotations = ToolAnnotations::read_only("Search by name");
}

#[async_trait]
impl AsyncTool<SearchToolContext> for NameSearch {
    type Output = SearchToolResponse;

    #[tracing::instrument(skip_all, fields(user_id=?(*request_context.user_id).as_ref()), err)]
    async fn call(
        &self,
        search_context: ServiceContext<SearchToolContext>,
        request_context: RequestContext,
    ) -> ToolResult<Self::Output> {
        tracing::info!(self=?self, "Name search params");

        if self.name.trim().is_empty() {
            return Err(ToolCallError {
                description: "name must not be empty".to_string(),
                internal_error: anyhow::anyhow!("name must not be empty"),
            });
        }

        let tag_filter_active = self.tags.as_ref().is_some_and(|tags| !tags.is_empty());
        let mut tag_sets = None;
        let mut tag_option_ids = Vec::new();
        if let Some(filters) = self.tags.as_deref().filter(|t| !t.is_empty()) {
            let (sets, option_ids) = resolve_tag_filters(
                &search_context,
                (*request_context.user_id).as_ref(),
                filters,
                self.tags_match,
            )
            .await?;
            tag_sets = Some(sets);
            tag_option_ids = option_ids;
        }

        let base_filters = EntityFilters {
            tag_option_ids,
            tag_filter_mode: tag_filter_mode(self.tags_match),
            ..Default::default()
        };
        let non_channel_types = if self.entity_types.is_empty() {
            vec![
                SearchEntityType::Documents,
                SearchEntityType::AgentSessions,
                SearchEntityType::Projects,
                SearchEntityType::CallRecords,
            ]
        } else {
            self.entity_types
                .iter()
                .filter(|entity_type| **entity_type != SearchEntityType::Channels)
                .cloned()
                .collect()
        };
        let include_channels = !tag_filter_active
            && (self.entity_types.is_empty()
                || self.entity_types.contains(&SearchEntityType::Channels));
        let search_request = (!non_channel_types.is_empty()).then(|| UnifiedSearchRequest {
            query: self.name.to_owned(),
            match_type: self.match_type.into(),
            filters: search_filters(&non_channel_types, base_filters),
            search_on: models_search::SearchOn::Name,
            collapse: None,
        });
        let channel_request = include_channels.then(|| ChannelNameSearchRequest {
            query: self.name.to_owned(),
            match_type: self.match_type.into(),
        });
        let user_id = (*request_context.user_id).as_ref();

        let (response, channel_response) = tokio::try_join!(
            async {
                let Some(search_request) = search_request else {
                    return Ok(None);
                };
                search_context
                    .search_client
                    .search_unified(user_id, search_request, None, PAGE_SIZE)
                    .await
                    .map(Some)
                    .map_err(|e| ToolCallError {
                        description: format!("failed to perform name search: {e}"),
                        internal_error: e,
                    })
            },
            async {
                let Some(channel_request) = channel_request else {
                    return Ok(None);
                };
                search_context
                    .search_client
                    .search_channel_names(user_id, channel_request, None, PAGE_SIZE)
                    .await
                    .map(Some)
                    .map_err(|e| ToolCallError {
                        description: format!("failed to search channel names: {e}"),
                        internal_error: e,
                    })
            },
        )?;

        // Drop the chat the agent is currently running inside so it never
        // surfaces itself in its own search results.
        let mut results = response
            .map(|response| response.results)
            .unwrap_or_default();
        results.extend(
            channel_response
                .into_iter()
                .flat_map(|response| response.results)
                .map(UnifiedSearchResponseItem::Channel),
        );
        results.sort_by(|a, b| {
            b.updated_at()
                .cmp(&a.updated_at())
                .then_with(|| b.entity_id().cmp(&a.entity_id()))
        });
        results.truncate(PAGE_SIZE as usize);
        if let Some(self_chat_id) = search_context.self_chat_id {
            results.retain(|item| item.entity_id() != self_chat_id);
        }

        let results = annotate_result_tags(
            &search_context,
            (*request_context.user_id).as_ref(),
            results,
            tag_sets,
        )
        .await?;

        Ok(SearchToolResponse { results })
    }
}
