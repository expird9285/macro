use super::*;

use crate::search::builder::updated_at_sort;
use chrono::Utc;
use models_search_cursor::SearchMethodCursor;
use opensearch_query_builder::ToOpenSearchJson;

#[test]
fn author_only_channel_match_without_highlight_keeps_deep_link() {
    let channel_id = uuid::Uuid::now_v7();
    let message_id = uuid::Uuid::now_v7();
    for thread in [None, Some(uuid::Uuid::now_v7())] {
        let source = UnifiedSearchIndex::from_matched(
            &["channels".into()],
            serde_json::json!({
                "entity_id": channel_id, "message_id": message_id,
                "thread_id": thread.unwrap_or(message_id), "channel_type": "private",
                "sender_id": "macro|system@macro.com", "mentions": [],
                "imported_author": "quartzarchivebot"
            }),
        )
        .unwrap();
        let hits = expand_hit_into_search_hits(Hit {
            source,
            index: "channels_v2".into(),
            matched_queries: vec!["channels".into()],
            score: Some(1.0),
            highlight: None,
            inner_hits: None,
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity_id, channel_id);
        let Some(SearchGotoContent::Channels(goto)) = &hits[0].goto else {
            panic!("channel hit must retain a deep link without highlights");
        };
        assert_eq!(goto.channel_message_id, message_id);
        assert_eq!(goto.thread_id, thread);
        assert_eq!(goto.sender_id, "macro|system@macro.com");
    }
}

#[test]
fn agent_sessions_expand_and_paginate_as_entities_not_messages() {
    let session_id = uuid::Uuid::from_u128(1);
    let next_id = uuid::Uuid::from_u128(2);
    let session = Hit {
        index: "agent_sessions".into(),
        matched_queries: vec!["agent_sessions".into()],
        score: Some(1.0),
        source: serde_json::json!({
            "agent_session_id": session_id,
            "updated_at_millis": 1783000000000_i64,
        }),
        highlight: None,
        inner_hits: Some(serde_json::json!({
            "agent_term_0": {"hits": {"hits": [
                {"_id": "user", "_source": {"message_turn": 0, "author": "user"},
                 "highlight": {"content": ["<macro_em>match</macro_em>"]}},
                {"_id": "agent", "_source": {"message_turn": 0, "author": "agent"},
                 "highlight": {"content": ["<macro_em>match</macro_em>"]}}
            ]}}
        })),
    };
    let (hits, cursor) = paginate_unified_hits(
        vec![session, doc_hit_with_chunks(next_id, 1782999999000, 1)],
        1,
    );
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|hit| hit.entity_id == session_id));
    let SearchCursorOption::NotDone(Some(SearchMethodCursor::UpdatedAt { entity_id, .. })) = cursor
    else {
        panic!("expected a continuation anchored on the session")
    };
    assert_eq!(entity_id, session_id);
    let split = hits.into_iter().split_search_response();
    assert_eq!(split.agent_session.len(), 2);
    assert!(
        split.chat.is_empty(),
        "agent sessions are not legacy DCS chats"
    );
}

#[test]
fn expand_document_name_highlight_yields_name_hit_without_goto() {
    use std::collections::HashMap;
    use uuid::Uuid;

    let entity_id = "00000000-0000-0000-0000-000000000001"
        .parse::<Uuid>()
        .unwrap();
    // A parent document whose name matched: a top-level `document_name`
    // highlight, no inner_hits (the content branch did not match).
    let hit = Hit {
        index: "documents_v2".to_string(),
        matched_queries: vec!["documents".to_string()],
        score: Some(2.0),
        source: UnifiedSearchIndex::Document(DocumentIndex {
            entity_id,
            document_name: "Q3 Report".to_string(),
            owner_id: "alice".to_string(),
            file_type: "md".to_string(),
            updated_at_millis: Some(1_779_000_000_000),
        }),
        highlight: Some(HashMap::from([(
            "document_name".to_string(),
            vec!["Q3 <macro_em>Rep</macro_em>ort".to_string()],
        )])),
        inner_hits: None,
    };

    let results = expand_hit_into_search_hits(hit);
    assert_eq!(results.len(), 1, "name-only match yields a single hit");
    assert_eq!(results[0].entity_id, entity_id);
    assert!(
        results[0].goto.is_none(),
        "a name match carries no chunk goto so grouping treats it as a name result"
    );
    assert_eq!(
        results[0].highlight.name.as_deref(),
        Some("Q3 <macro_em>Rep</macro_em>ort")
    );
}

#[test]
fn test_deserialization() -> anyhow::Result<()> {
    let json = serde_json::json!({
      "took": 20,
      "timed_out": false,
      "_shards": {
        "total": 15,
        "successful": 15,
        "skipped": 0,
        "failed": 0
      },
      "hits": {
        "total": {
          "value": 739,
          "relation": "eq"
        },
        "max_score": 27.399992,
        "hits": [
          {
            "_index": "documents",
            "_id": "013021c7-5e14-4aab-855e-01413fff85dc:77",
            "_score": 18.047625,
            "_source": {
              "document_name": "Rust for Rustaceans Idiomatic Programming for Experienced Developers (1)",
              "updated_at": 1752523635,
              "updated_at_millis": 1752523635000_i64,
              "file_type": "pdf",
              "owner_id": "macro|teo@macro.com",
              "document_id": "013021c7-5e14-4aab-855e-01413fff85dc",
              "document_version_id": 10091,
              "entity_id": "013021c7-5e14-4aab-855e-01413fff85dc",
              "content": "74   Chapter 6\r\n[patch.crates-io]\r\n# use a local (presumably modified) source\r\nregex = { path = \"/home/jon/regex\" }\r\n# use a modification on a git branch\r\nserde = { git = \"https://github.com/serde-rs/serde.git\", branch = \"faster\" }\r\n# patch a git dependency\r\n[patch.'https://github.com/jonhoo/project.git']\r\nproject = { path = \"/home/jon/project\" }\r\nListing 6-6: Overriding dependency sources in Cargo.toml using [patch]\r\nEven if you patch a dependency, Cargo takes care to check the crate \r\nversions so that you don’t accidentally end up patching the wrong major version of a crate. If you for some reason transitively depend on multiple major \r\nversions of the same crate, you can patch each one by giving them distinct \r\nidentifiers, as shown in Listing 6-7.\r\n[patch.crates-io]\r\nnom4 = { path = \"/home/jon/nom4\", package = \"nom\" }\r\nnom5 = { path = \"/home/jon/nom5\", package = \"nom\" }\r\nListing 6-7: Overriding multiple versions of the same crate in Cargo.toml using [patch]\r\nCargo will look at the Cargo.toml inside each path, realize that /nom4\r\ncontains major version 4 and that /nom5 contains major version 5, and patch \r\nthe two versions appropriately. The package keyword tells Cargo to look for a \r\ncrate by the name nom in both cases instead of using the dependency identifiers (the part on the left) as it does by default. You can use package this way \r\nin your regular dependencies as well to rename a dependency!\r\nKeep in mind that patches are not taken into account in the package \r\nthat’s uploaded when you publish a crate. A crate that depends on your \r\ncrate will use only its own [patch] section (which may be empty), not that of \r\nyour crate!\r\nCR ATES VS. PACKAGES\r\nYou may wonder what the difference between a package and a crate is. \r\nThese two terms are often used interchangeably in informal contexts, but they \r\nalso have specific definitions that vary depending on whether you’re talking \r\nabout the Rust compiler, Cargo, crates.io, or something else. I personally think \r\nof a crate as a Rust module hierarchy starting at a root .rs file (one where you \r\ncan use crate-level attributes like #![feature])—usually something like lib.rs or \r\nmain.rs. In contrast, a package is a collection of crates and metadata, so essentially all that’s described by a Cargo.toml file. That may include a library crate, \r\nmultiple binary crates, some integration test crates, and maybe even multiple \r\nworkspace members that themselves have Cargo.toml files.\r\nIntermediate Rust (Early Access) © 2022 by Jon Gjengset ",
              "node_id": "77"
            },
            "highlight": {
              "content": [
                "local (presumably modified) source\r\nregex = { path = \"/home/jon/regex\" }\r\n# use a modification on a <macro_em>git</macro_em>",
                "branch\r\nserde = { <macro_em>git</macro_em> = \"https://<macro_em>github.com</macro_em>/serde-rs/serde.git\", branch = \"faster\" }\r\n# patch a <macro_em>git</macro_em>",
                "https://<macro_em>github.com</macro_em>/jonhoo/project.git']\r\nproject = { path = \"/home/jon/project\" }\r\nListing 6-6: Overriding"
              ]
            }
          },
          {
            "_index": "documents",
            "_id": "71ff8822-b421-4cab-b23e-f65355bf90d4:77",
            "_score": 17.689163,
            "_source": {
              "document_name": "Rust for Rustaceans Idiomatic Programming for Experienced Developers (1)",
              "updated_at": 1752522588,
              "updated_at_millis": 1752522588000_i64,
              "file_type": "pdf",
              "owner_id": "macro|teo@macro.com",
              "document_id": "71ff8822-b421-4cab-b23e-f65355bf90d4",
              "document_version_id": 511386,
              "entity_id": "71ff8822-b421-4cab-b23e-f65355bf90d4",
              "content": "74   Chapter 6\r\n[patch.crates-io]\r\n# use a local (presumably modified) source\r\nregex = { path = \"/home/jon/regex\" }\r\n# use a modification on a git branch\r\nserde = { git = \"https://github.com/serde-rs/serde.git\", branch = \"faster\" }\r\n# patch a git dependency\r\n[patch.'https://github.com/jonhoo/project.git']\r\nproject = { path = \"/home/jon/project\" }\r\nListing 6-6: Overriding dependency sources in Cargo.toml using [patch]\r\nEven if you patch a dependency, Cargo takes care to check the crate \r\nversions so that you don’t accidentally end up patching the wrong major version of a crate. If you for some reason transitively depend on multiple major \r\nversions of the same crate, you can patch each one by giving them distinct \r\nidentifiers, as shown in Listing 6-7.\r\n[patch.crates-io]\r\nnom4 = { path = \"/home/jon/nom4\", package = \"nom\" }\r\nnom5 = { path = \"/home/jon/nom5\", package = \"nom\" }\r\nListing 6-7: Overriding multiple versions of the same crate in Cargo.toml using [patch]\r\nCargo will look at the Cargo.toml inside each path, realize that /nom4\r\ncontains major version 4 and that /nom5 contains major version 5, and patch \r\nthe two versions appropriately. The package keyword tells Cargo to look for a \r\ncrate by the name nom in both cases instead of using the dependency identifiers (the part on the left) as it does by default. You can use package this way \r\nin your regular dependencies as well to rename a dependency!\r\nKeep in mind that patches are not taken into account in the package \r\nthat’s uploaded when you publish a crate. A crate that depends on your \r\ncrate will use only its own [patch] section (which may be empty), not that of \r\nyour crate!\r\nCR ATES VS. PACKAGES\r\nYou may wonder what the difference between a package and a crate is. \r\nThese two terms are often used interchangeably in informal contexts, but they \r\nalso have specific definitions that vary depending on whether you’re talking \r\nabout the Rust compiler, Cargo, crates.io, or something else. I personally think \r\nof a crate as a Rust module hierarchy starting at a root .rs file (one where you \r\ncan use crate-level attributes like #![feature])—usually something like lib.rs or \r\nmain.rs. In contrast, a package is a collection of crates and metadata, so essentially all that’s described by a Cargo.toml file. That may include a library crate, \r\nmultiple binary crates, some integration test crates, and maybe even multiple \r\nworkspace members that themselves have Cargo.toml files.\r\nIntermediate Rust (Early Access) © 2022 by Jon Gjengset ",
              "node_id": "77"
            },
            "highlight": {
              "content": [
                "local (presumably modified) source\r\nregex = { path = \"/home/jon/regex\" }\r\n# use a modification on a <macro_em>git</macro_em>",
                "branch\r\nserde = { <macro_em>git</macro_em> = \"https://<macro_em>github.com</macro_em>/serde-rs/serde.git\", branch = \"faster\" }\r\n# patch a <macro_em>git</macro_em>",
                "https://<macro_em>github.com</macro_em>/jonhoo/project.git']\r\nproject = { path = \"/home/jon/project\" }\r\nListing 6-6: Overriding"
              ]
            }
          }
        ]
      }
    });

    let _: DefaultSearchResponse<serde_json::Value> = serde_json::from_value(json)?;

    Ok(())
}

#[test]
fn test_build_unified_search_request_content() -> anyhow::Result<()> {
    let entity_id = uuid::Uuid::new_v4();
    let time = Utc::now();

    let unified_search_args = UnifiedSearchArgs {
        search_indices: vec![
            OpenSearchEntityType::Documents,
            OpenSearchEntityType::Channels,
            OpenSearchEntityType::Chats,
        ]
        .into_iter()
        .collect(),
        user_id: "user".to_string(),
        page: 1,
        page_size: 20,
        match_type: "exact".to_string(),

        collapse: true,
        document_search_args: UnifiedDocumentSearchArgs {
            terms: vec!["test".to_string()],
            document_ids: vec!["id1".to_string(), "id2".to_string()],
            ids_only: false,
            ..Default::default()
        },
        channel_message_search_args: UnifiedChannelMessageSearchArgs {
            terms: vec!["test".to_string()],
            channel_ids: vec!["id1".to_string(), "id2".to_string()],
            thread_ids: vec!["id1".to_string(), "id2".to_string()],
            mentions: vec!["id1".to_string(), "id2".to_string()],
            sender_ids: vec!["id1".to_string(), "id2".to_string()],
        },
        chat_search_args: UnifiedChatSearchArgs {
            terms: vec!["test".to_string()],
            chat_ids: vec!["id1".to_string(), "id2".to_string()],
            role: vec!["id1".to_string(), "id2".to_string()],
            ids_only: false,
            ..Default::default()
        },
        call_record_search_args: UnifiedCallRecordSearchArgs::default(),
        project_search_args: UnifiedProjectSearchArgs::default(),
        agent_session_search_args: Default::default(),
        cursor: SearchCursorOption::NotDone(Some(SearchMethodCursor::UpdatedAt {
            entity_id,
            updated_at: time,
        })),
    };

    let result = build_unified_search_request(&unified_search_args)?;

    // Documents and chats are join-shape (has_child); compute their
    // sub-queries from the builders rather than inlining the has_child
    // JSON. Channels are flat and stay inline below.
    let doc_query_builder =
        DocumentQueryBuilder::from(DocumentSearchArgs::from(unified_search_args.clone()));
    let doc_should = {
        let mut builder = doc_query_builder.build_bool_query()?;
        builder.name(OpenSearchEntityType::Documents.query_name());
        QueryType::from(builder.build()).to_json()
    };
    let chat_query_builder =
        ChatQueryBuilder::from(ChatSearchArgs::from(unified_search_args.clone()));
    let chat_should = {
        let mut builder = chat_query_builder.build_bool_query()?;
        builder.name(OpenSearchEntityType::Chats.query_name());
        QueryType::from(builder.build()).to_json()
    };

    let expected = serde_json::json!({
      "collapse": {
        "field": "entity_id"
      },
      "highlight": {
        "fields": {
          "content": { "number_of_fragments": 1, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" },
          "document_name": { "number_of_fragments": 0, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" },
          "name": { "number_of_fragments": 0, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" }
        },
        "require_field_match": true,
        "max_analyzer_offset": 999999
      },
      "query": {
        "bool": {
          "minimum_should_match": 1,
          "should": [
            doc_should,
            {
              "bool": {
                "filter": [
                  {
                    "terms": {
                      "entity_id": ["id1", "id2"]
                    }
                  },
                  {
                    "term": {
                      "_index": "channels"
                    }
                  },
                  {
                    "terms": {
                      "thread_id": ["id1", "id2"]
                    }
                  },
                  {
                    "terms": {
                      "mentions": ["id1", "id2"]
                    }
                  },
                  {
                    "terms": {
                      "sender_id": ["id1", "id2"]
                    }
                  }
                ],
                "_name": "channels",
                "must": [
                  {
                    "bool": {
                      "minimum_should_match": 1,
                      "should": [
                        {
                          "match_phrase": {
                            "content": "test"
                          }
                        }
                      ]
                    }
                  }
                ]
              }
            },
            chat_should
          ]
        }
      },
      "search_after": [time.timestamp_millis(), entity_id.to_string()],
      "size": 21,
      "sort": updated_at_sort().iter().map(|s| s.to_json()).collect::<Vec<_>>(),
    });

    assert_eq!(result.to_json(), expected);

    let unified_search_args = UnifiedSearchArgs {
        search_indices: vec![
            OpenSearchEntityType::Documents,
            OpenSearchEntityType::Channels,
            OpenSearchEntityType::Chats,
        ]
        .into_iter()
        .collect(),
        user_id: "user".to_string(),
        page: 1,
        page_size: 20,
        match_type: "exact".to_string(),

        collapse: true,
        document_search_args: UnifiedDocumentSearchArgs {
            terms: vec!["test".to_string()],
            document_ids: vec!["id1".to_string(), "id2".to_string()],
            ids_only: false,
            ..Default::default()
        },
        channel_message_search_args: UnifiedChannelMessageSearchArgs {
            terms: vec!["test".to_string()],
            channel_ids: vec!["id1".to_string(), "id2".to_string()],
            thread_ids: vec!["id1".to_string(), "id2".to_string()],
            mentions: vec!["id1".to_string(), "id2".to_string()],
            sender_ids: vec!["id1".to_string(), "id2".to_string()],
        },
        chat_search_args: UnifiedChatSearchArgs {
            terms: vec!["test".to_string()],
            chat_ids: vec!["id1".to_string(), "id2".to_string()],
            role: vec!["id1".to_string(), "id2".to_string()],
            ids_only: false,
            ..Default::default()
        },
        call_record_search_args: UnifiedCallRecordSearchArgs::default(),
        project_search_args: UnifiedProjectSearchArgs::default(),
        agent_session_search_args: Default::default(),
        cursor: SearchCursorOption::NotDone(Some(SearchMethodCursor::UpdatedAt {
            entity_id,
            updated_at: time,
        })),
    };

    let result = build_unified_search_request(&unified_search_args)?;

    assert_eq!(result.to_json(), expected);

    Ok(())
}

#[test]
fn test_build_unified_search_request_single_index() -> anyhow::Result<()> {
    let unified_search_args = UnifiedSearchArgs {
        search_indices: vec![OpenSearchEntityType::Documents].into_iter().collect(),
        user_id: "user".to_string(),
        page: 1,
        page_size: 20,
        match_type: "exact".to_string(),

        collapse: true,
        document_search_args: UnifiedDocumentSearchArgs {
            terms: vec!["test".to_string()],
            document_ids: vec!["id1".to_string(), "id2".to_string()],
            ids_only: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let result = build_unified_search_request(&unified_search_args)?;

    // Documents are join-shape (has_child); compute the sub-query from the
    // builder rather than inlining the has_child JSON.
    let doc_query_builder =
        DocumentQueryBuilder::from(DocumentSearchArgs::from(unified_search_args.clone()));
    let doc_should = {
        let mut builder = doc_query_builder.build_bool_query()?;
        builder.name(OpenSearchEntityType::Documents.query_name());
        QueryType::from(builder.build()).to_json()
    };

    let expected = serde_json::json!(
          {
      "collapse": {
        "field": "entity_id"
      },
      "highlight": {
        "fields": {
          "content": { "number_of_fragments": 1, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" },
          "document_name": { "number_of_fragments": 0, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" },
          "name": { "number_of_fragments": 0, "post_tags": ["</macro_em>"], "pre_tags": ["<macro_em>"], "type": "plain" }
        },
        "require_field_match": true,
        "max_analyzer_offset": 999999
      },
      "query": {
        "bool": {
          "minimum_should_match": 1,
          "should": [
            doc_should
          ]
        }
      },
      "size": 21,
      "sort": updated_at_sort().iter().map(|s| s.to_json()).collect::<Vec<_>>(),
    });

    assert_eq!(result.to_json(), expected);

    Ok(())
}

#[test]
fn test_build_unified_search_request_empty_indices() -> anyhow::Result<()> {
    let unified_search_args = UnifiedSearchArgs {
        search_indices: vec![].into_iter().collect(),
        ..Default::default()
    };

    let err = build_unified_search_request(&unified_search_args).unwrap_err();

    assert_eq!(err, OpensearchClientError::EmptySearchIndices);

    Ok(())
}

#[test]
fn test_thread_sort_is_thread_id_then_message_id_field_sorts() {
    use crate::search::builder::thread_sort;
    let json: Vec<serde_json::Value> = thread_sort().iter().map(|s| s.to_json()).collect();

    assert_eq!(json.len(), 2);
    assert_eq!(json[0]["thread_id"]["order"], "desc");
    assert_eq!(json[0]["thread_id"]["unmapped_type"], "keyword");
    assert_eq!(json[1]["message_id"]["order"], "desc");
    assert_eq!(json[1]["message_id"]["unmapped_type"], "keyword");
}

/// A join-shape parent document whose content matched on `chunk_count` chunks,
/// shaped like the `inner_hits` block OpenSearch returns for a `has_child`
/// content query.
fn doc_hit_with_chunks(
    entity_id: uuid::Uuid,
    updated_at_millis: i64,
    chunk_count: usize,
) -> Hit<serde_json::Value> {
    let chunks: Vec<serde_json::Value> = (0..chunk_count)
        .map(|i| {
            serde_json::json!({
                "_id": format!("{entity_id}-chunk-{i}"),
                "_score": 1.0,
                "_source": { "node_id": format!("node-{i}"), "raw_content": "match" },
                "highlight": { "content": ["<macro_em>match</macro_em>"] },
            })
        })
        .collect();

    Hit {
        index: "documents_v2".to_string(),
        matched_queries: vec!["documents".to_string()],
        score: Some(1.0),
        source: serde_json::json!({
            "entity_id": entity_id,
            "document_name": "Doc",
            "owner_id": "alice",
            "file_type": "md",
            "updated_at_millis": updated_at_millis,
        }),
        highlight: None,
        inner_hits: Some(serde_json::json!({ "term_0": { "hits": { "hits": chunks } } })),
    }
}

#[test]
fn paginate_counts_parent_entities_not_inner_hits() {
    // Two documents, each matched on many content chunks. The expanded hit count
    // far exceeds page_size, but there are only two entities — the page is not
    // full, so no spurious "more" cursor is minted and every chunk is returned.
    let d1 = "00000000-0000-0000-0000-000000000001"
        .parse::<uuid::Uuid>()
        .unwrap();
    let d2 = "00000000-0000-0000-0000-000000000002"
        .parse::<uuid::Uuid>()
        .unwrap();
    let hits = vec![
        doc_hit_with_chunks(d1, 1_779_000_100_000, 30),
        doc_hit_with_chunks(d2, 1_779_000_050_000, 30),
    ];

    let (results, cursor) = paginate_unified_hits(hits, 10);

    assert_eq!(
        results.len(),
        60,
        "all chunks of both documents are returned"
    );
    assert!(
        cursor.is_done(),
        "two entities under a page_size of 10 must not report more results, got {cursor:?}"
    );
}

#[test]
fn paginate_has_more_when_parent_count_exceeds_page_size() {
    // page_size + 1 distinct documents: the extra one signals more, is dropped
    // before expansion, and the cursor anchors on the last *included* document.
    let d1 = "00000000-0000-0000-0000-000000000001"
        .parse::<uuid::Uuid>()
        .unwrap();
    let d2 = "00000000-0000-0000-0000-000000000002"
        .parse::<uuid::Uuid>()
        .unwrap();
    let d3 = "00000000-0000-0000-0000-000000000003"
        .parse::<uuid::Uuid>()
        .unwrap();
    let hits = vec![
        doc_hit_with_chunks(d1, 1_779_000_300_000, 5),
        doc_hit_with_chunks(d2, 1_779_000_200_000, 5),
        doc_hit_with_chunks(d3, 1_779_000_100_000, 5),
    ];

    let (results, cursor) = paginate_unified_hits(hits, 2);

    assert_eq!(
        results.len(),
        10,
        "only the two included docs' chunks remain"
    );
    assert!(
        results
            .iter()
            .all(|h| h.entity_id == d1 || h.entity_id == d2),
        "the third document must be dropped entirely, not partially"
    );
    match cursor {
        SearchCursorOption::NotDone(Some(c)) => {
            let (id, _) = c.as_updated_at().expect("UpdatedAt cursor");
            assert_eq!(id, d2, "cursor anchors on the last included entity");
        }
        _ => panic!("expected NotDone cursor, got {cursor:?}"),
    }
}

#[test]
fn paginate_page_size_zero_terminates() {
    // page_size 0 passes request validation, and the query fetches size(1) so a
    // hit can return. The page is empty (take(0)) with no anchor, so the cursor
    // must be Done — a NotDone here would loop the caller on empty pages.
    let d1 = "00000000-0000-0000-0000-000000000001"
        .parse::<uuid::Uuid>()
        .unwrap();
    let hits = vec![doc_hit_with_chunks(d1, 1_779_000_100_000, 5)];

    let (results, cursor) = paginate_unified_hits(hits, 0);

    assert!(results.is_empty(), "page_size 0 yields no results");
    assert!(
        cursor.is_done(),
        "no anchor to resume from must terminate, not loop, got {cursor:?}"
    );
}

#[test]
fn an_unknown_clause_is_rejected_not_guessed() {
    let source = serde_json::json!({
        "entity_id": "0197a863-0000-7000-8000-0000000000c1",
        "name": "Something",
        "owner_id": "macro|user@user.com"
    });
    assert!(
        UnifiedSearchIndex::from_matched(&["reminders".to_string()], source.clone()).is_err(),
        "a hit matching no known entity clause must not be filed as some other entity"
    );
    assert!(
        UnifiedSearchIndex::from_matched(&[], source).is_err(),
        "a hit that reported no clause names at all has no discriminator"
    );
}

#[test]
fn an_unplaceable_hit_is_dropped_without_failing_the_page() {
    // One good hit, one from an index we do not know. The page survives.
    let good = Hit {
        index: "projects_v1".to_string(),
        matched_queries: vec!["projects".to_string()],
        score: Some(1.0),
        source: serde_json::json!({
            "entity_id": "0197a863-0000-7000-8000-000000000001",
            "name": "Mobile Redesign",
            "owner_id": "macro|user@user.com",
            "updated_at_millis": 1783000000000_i64
        }),
        highlight: None,
        inner_hits: None,
    };
    let unplaceable = Hit {
        index: "reminders_v1".to_string(),
        matched_queries: vec!["reminders".to_string()],
        score: Some(1.0),
        source: serde_json::json!({ "entity_id": "0197a863-0000-7000-8000-000000000009" }),
        highlight: None,
        inner_hits: None,
    };

    let (hits, _) = paginate_unified_hits(vec![good, unplaceable], 10);
    assert_eq!(hits.len(), 1, "the unplaceable hit is dropped, not fatal");
    assert_eq!(hits[0].entity_type, SearchEntityType::Projects);
}

/// A response mixing every entity type, the way a search across all indices
/// actually comes back. Each hit must land on its own type — this is the case
/// the untagged enum could get wrong and the reason dispatch is explicit.
#[test]
fn a_mixed_response_routes_every_entity_to_its_own_type() -> anyhow::Result<()> {
    fn hit(index: &str, matched: &str, source: serde_json::Value) -> Hit<serde_json::Value> {
        Hit {
            index: index.to_string(),
            matched_queries: vec![matched.to_string()],
            score: Some(1.0),
            source,
            highlight: None,
            inner_hits: None,
        }
    }

    let id = |n: u8| format!("0197a863-0000-7000-8000-0000000000{n:02}");

    let hits = vec![
        hit(
            "channels_v2",
            "channels",
            serde_json::json!({
                "entity_id": id(1), "channel_type": "standard", "org_id": null,
                "message_id": id(2), "thread_id": id(2), "sender_id": "macro|a",
                "mentions": [], "updated_at_millis": 1_783_000_000_000_i64
            }),
        ),
        hit(
            "documents_v2",
            "documents",
            serde_json::json!({
                "entity_id": id(3), "document_name": "Spec", "owner_id": "macro|a",
                "file_type": "md", "updated_at_millis": 1_783_000_000_001_i64
            }),
        ),
        hit(
            "chats_v2",
            "chats",
            serde_json::json!({
                "entity_id": id(4), "user_id": "macro|a", "title": "Chat",
                "updated_at_millis": 1_783_000_000_002_i64
            }),
        ),
        hit(
            "call_records_v2",
            "call_records",
            serde_json::json!({
                "entity_id": id(8), "channel_id": id(9), "participant_ids": [],
                "name": "Standup call", "started_at_millis": 1_783_000_000_004_i64
            }),
        ),
        hit(
            "projects_v1",
            "projects",
            serde_json::json!({
                "entity_id": id(10), "name": "Redesign", "owner_id": "macro|a",
                "updated_at_millis": 1_783_000_000_005_i64
            }),
        ),
    ];

    let (results, _) = paginate_unified_hits(hits, 50);

    // Every hit survived: none was dropped as unplaceable.
    assert_eq!(results.len(), 5, "got {results:?}");

    let by_id: std::collections::HashMap<uuid::Uuid, SearchEntityType> = results
        .iter()
        .map(|hit| (hit.entity_id, hit.entity_type.clone()))
        .collect();
    for (n, expected) in [
        (1u8, SearchEntityType::Channels),
        (3, SearchEntityType::Documents),
        (4, SearchEntityType::Chats),
        (8, SearchEntityType::CallRecords),
        (10, SearchEntityType::Projects),
    ] {
        let entity_id = id(n).parse::<uuid::Uuid>()?;
        assert_eq!(
            by_id.get(&entity_id),
            Some(&expected),
            "entity {n} routed to the wrong type"
        );
    }

    // The downstream split must bucket them the same way.
    let split = results.into_iter().split_search_response();
    assert_eq!(split.channel_message.len(), 1);
    assert_eq!(split.document.len(), 1);
    assert_eq!(split.chat.len(), 1);
    assert_eq!(split.call_record.len(), 1);
    assert_eq!(split.project.len(), 1);
    Ok(())
}

/// Searching every index must name every clause, or a hit from an unnamed one
/// would come back with no discriminator and be dropped.
#[test]
fn every_searched_index_contributes_a_named_clause() -> anyhow::Result<()> {
    use strum::IntoEnumIterator as _;

    let args = UnifiedSearchArgs {
        search_indices: OpenSearchEntityType::iter().collect(),
        agent_session_search_args: super::super::agent_sessions::AgentSessionSearchArgs {
            terms: vec!["test".into()],
            session_ids: vec![uuid::Uuid::from_u128(1).to_string()],
            ..Default::default()
        },
        user_id: "user".to_string(),
        page_size: 10,
        match_type: "exact".to_string(),
        document_search_args: UnifiedDocumentSearchArgs {
            terms: vec!["test".to_string()],
            ..Default::default()
        },
        channel_message_search_args: UnifiedChannelMessageSearchArgs {
            terms: vec!["test".to_string()],
            channel_ids: vec!["c1".to_string()],
            ..Default::default()
        },
        chat_search_args: UnifiedChatSearchArgs {
            terms: vec!["test".to_string()],
            ..Default::default()
        },
        call_record_search_args: UnifiedCallRecordSearchArgs {
            terms: vec!["test".to_string()],
            call_ids: vec!["r1".to_string()],
            ids_only: true,
            ..Default::default()
        },
        project_search_args: UnifiedProjectSearchArgs {
            terms: vec!["test".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let json = build_unified_search_request(&args)?.to_json();
    let should = json["query"]["bool"]["should"]
        .as_array()
        .expect("a should clause per searched index");

    let names: std::collections::HashSet<&str> = should
        .iter()
        .filter_map(|clause| clause["bool"]["_name"].as_str())
        .collect();

    for entity in OpenSearchEntityType::iter() {
        assert!(
            names.contains(entity.query_name()),
            "{} contributed an unnamed clause; its hits would be undispatchable",
            entity.query_name()
        );
    }
    assert_eq!(
        names.len(),
        should.len(),
        "every should clause must be named, got {names:?} across {} clauses",
        should.len()
    );
    Ok(())
}

#[test]
fn project_hit_deserializes_and_converts() -> anyhow::Result<()> {
    // A project doc as the flat projects index returns it, parsed the way the
    // client parses a real response: raw `_source`, then dispatched on
    // `_index`.
    let json = serde_json::json!({
      "took": 3,
      "timed_out": false,
      "_shards": { "total": 3, "successful": 3, "skipped": 0, "failed": 0 },
      "hits": {
        "total": { "value": 1, "relation": "eq" },
        "max_score": 4.2,
        "hits": [
          {
            "_index": "projects_v1",
            "_id": "0197a863-0000-7000-8000-000000000001",
            "_score": 4.2,
            "_source": {
              "entity_id": "0197a863-0000-7000-8000-000000000001",
              "name": "Mobile Redesign",
              "owner_id": "macro|user@user.com",
              "parent_project_id": "0197a863-0000-7000-8000-000000000002",
              "created_at_millis": 1770000000000_i64,
              "updated_at_millis": 1783000000000_i64,
              "properties": [
                { "definition_id": "0197a863-0000-7000-8000-00000000000a", "values": ["opt-1"] }
              ]
            },
            "matched_queries": ["projects"],
            "highlight": {
              "name": ["<macro_em>Mobile</macro_em> Redesign"]
            }
          }
        ]
      }
    });

    let result: DefaultSearchResponse<serde_json::Value> = serde_json::from_value(json)?;
    assert_eq!(result.hits.hits.len(), 1);
    assert!(matches!(
        UnifiedSearchIndex::from_matched(
            &result.hits.hits[0].matched_queries,
            result.hits.hits[0].source.clone()
        )?,
        UnifiedSearchIndex::Project(_)
    ));

    let (hits, cursor) = paginate_unified_hits(result.hits.hits, 10);
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.entity_type, SearchEntityType::Projects);
    assert_eq!(
        hit.entity_id,
        "0197a863-0000-7000-8000-000000000001".parse::<uuid::Uuid>()?
    );
    assert_eq!(
        hit.highlight.name.as_deref(),
        Some("<macro_em>Mobile</macro_em> Redesign")
    );
    assert!(hit.goto.is_none());
    assert_eq!(
        hit.updated_at,
        chrono::DateTime::from_timestamp(1_783_000_000, 0)
    );
    assert!(cursor.is_done());

    Ok(())
}
