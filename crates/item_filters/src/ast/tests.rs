use std::str::FromStr;

use super::*;
use crate::{CallFilters, CallStatus, ForeignEntityFilters, PropertyFilter};
use cool_asserts::assert_matches;
use model_file_type::FileType;
use serde_json::json;
use uuid::Uuid;

use super::properties;

#[test]
fn it_works_with_file_type() {
    let res: Result<Vec<_>, _> = ["pdf", "md", "txt", "html"]
        .into_iter()
        .map(FileType::from_str)
        .collect();

    assert_matches!(
        res.unwrap(),
        [FileType::Pdf, FileType::Md, FileType::Txt, FileType::Html]
    );
}

#[test]
fn it_expands_filters() {
    let document_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    let f = EntityFilters {
        document_filters: DocumentFilters {
            file_types: vec!["pdf".to_string(), "txt".to_string()],
            document_ids: vec![document_id.to_string()],
            project_ids: vec![project_id.to_string()],
            owners: vec!["macro|hello@test.com".to_string()],
            importance: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "&": [
            {
                "&": [
                    {
                        "&": [
                            {
                                "&": [
                                    {
                                        "|": [
                                            {
                                                "l": {
                                                    "ft": "pdf",
                                                }
                                            },
                                            {
                                                "l": {
                                                    "ft": "txt"
                                                }
                                            }
                                        ]
                                    },
                                    {
                                        "l": {
                                            "id": document_id
                                        }
                                    }
                                ]
                            },
                            {
                                "l": {
                                    "pid": project_id
                                }
                            }
                        ]
                    },
                    {
                        "l": {
                            "o": "macro|hello@test.com"
                        }
                    }
                ]
            },
            {
                "l": {
                    "imp": true
                }
            }
        ]
    });

    assert_eq!(json, exp);
}

#[test]
fn it_expands_file_associations() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            file_types: vec!["assoc:vector".to_string()],
            document_ids: vec![],
            project_ids: vec![],
            owners: vec![],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    println!("{}", serde_json::to_string_pretty(&json).unwrap());

    let exp = serde_json::json!({
      "|": [
        {
          "|": [
            {
              "|": [
                {
                  "|": [
                    {
                      "|": [
                        {
                          "|": [
                            {
                              "|": [
                                {
                                  "l": {
                                    "ft": "ai"
                                  }
                                },
                                {
                                  "l": {
                                    "ft": "eps"
                                  }
                                }
                              ]
                            },
                            {
                              "l": {
                                "ft": "ps"
                              }
                            }
                          ]
                        },
                        {
                          "l": {
                            "ft": "dxf"
                          }
                        }
                      ]
                    },
                    {
                      "l": {
                        "ft": "dwg"
                      }
                    }
                  ]
                },
                {
                  "l": {
                    "ft": "fig"
                  }
                }
              ]
            },
            {
              "l": {
                "ft": "psd"
              }
            }
          ]
        },
        {
          "l": {
            "ft": "psb"
          }
        }
      ]
    });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_other_association() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            file_types: vec!["assoc:other".to_string()],
            document_ids: vec![],
            project_ids: vec![],
            owners: vec![],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_string(&ast).unwrap();

    assert_eq!(json.trim(), include_str!("tests/other.json").trim());
}

#[test]
fn native_spreadsheets_have_an_explicit_filter_and_are_not_other_files() {
    assert_eq!(
        document::resolve_file_types("spreadsheet"),
        vec![FileType::Spreadsheet]
    );
    assert!(!document::resolve_file_types("assoc:other").contains(&FileType::Spreadsheet));
}

#[test]
fn it_expands_channel_types() {
    let f = EntityFilters {
        channel_filters: crate::ChannelFilters {
            channel_types: vec!["public".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "ChannelType": "public"
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn it_expands_is_participant() {
    let f = EntityFilters {
        channel_filters: crate::ChannelFilters {
            is_participant: Some(false),
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "IsParticipant": false
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn it_expands_multiple_channel_types() {
    let f = EntityFilters {
        channel_filters: crate::ChannelFilters {
            channel_types: vec!["public".to_string(), "direct_message".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // Should be OR of two channel types
    assert!(json.get("|").is_some());
}

#[test]
fn it_expands_channel_type_with_channel_id() {
    let channel_id = Uuid::new_v4();
    let f = EntityFilters {
        channel_filters: crate::ChannelFilters {
            channel_ids: vec![channel_id.to_string()],
            channel_types: vec!["private".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // Should be AND of channel_id and channel_type
    assert!(json.get("&").is_some());
}

#[test]
fn it_expands_channel_thread_filters() {
    let thread_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let f = EntityFilters {
        channel_thread_filters: crate::ChannelThreadFilters {
            thread_ids: vec![thread_id.to_string()],
            channel_ids: vec![channel_id.to_string()],
            root_sender_ids: vec!["macro|hello@test.com".to_string()],
            participant_ids: vec!["macro|participant@test.com".to_string()],
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_thread_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    assert!(json.get("&").is_some());
}

#[test]
fn it_expands_single_channel_thread_id() {
    let thread_id = Uuid::new_v4();
    let f = EntityFilters {
        channel_thread_filters: crate::ChannelThreadFilters {
            thread_ids: vec![thread_id.to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_thread_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "ThreadId": thread_id
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn it_expands_single_channel_thread_participant() {
    let f = EntityFilters {
        channel_thread_filters: crate::ChannelThreadFilters {
            participant_ids: vec!["macro|participant@test.com".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .channel_thread_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "Participant": "macro|participant@test.com"
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn notification_filter_dto_rejects_legacy_booleans_and_invalid_states() {
    for value in [
        json!({"done": true, "seen": false}),
        json!({"states": ["active"]}),
        json!({"states": [true]}),
    ] {
        assert!(serde_json::from_value::<crate::NotificationFilters>(value).is_err());
    }
}

#[test]
fn repeated_notification_states_do_not_build_an_unbounded_ast() {
    let filter = EntityFilters {
        document_filters: DocumentFilters {
            notification_filters: crate::NotificationFilters {
                states: vec![crate::NotificationState::Unseen; 10_000],
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let ast = EntityFilterAst::new_from_filters(filter).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(ast.document_filter.unwrap()).unwrap(),
        json!({"l": {"ns": "unseen"}})
    );
}

#[test]
fn it_expands_document_notification_filters() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            notification_filters: crate::NotificationFilters {
                states: vec![crate::NotificationState::Unseen],
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_string(&ast).unwrap();
    assert_eq!(json, r#"{"l":{"ns":"unseen"}}"#);
}

#[test]
fn it_expands_document_task_include_cbm_atm_nc_as_or() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            file_types: vec!["pdf".to_string()],
            task_filters: crate::TaskFilters {
                include_cbm_atm_nc: Some(true),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    assert!(
        json.get("|").is_some(),
        "include flag should OR with base filters"
    );
    let as_text = serde_json::to_string(&json).unwrap();
    assert!(as_text.contains(r#""cbm":true"#));
}

#[test]
fn task_include_cbm_atm_nc_false_is_noop() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            task_filters: crate::TaskFilters {
                include_cbm_atm_nc: Some(false),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).unwrap().is_none(),
        "false should be equivalent to not setting the include flag"
    );
}

#[test]
fn it_expands_single_property_select_option() {
    let prop_def_id = Uuid::new_v4();
    let option_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![option_id.to_string()],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    assert!(ast_result.properties_filter.is_some());
}

#[test]
fn it_expands_multiple_option_ids_as_or() {
    let prop_def_id = Uuid::new_v4();
    let option_a = Uuid::new_v4();
    let option_b = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![option_a.to_string(), option_b.to_string()],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // Two options should be OR'd together
    assert!(
        json.get("|").is_some(),
        "multiple option_ids should OR together"
    );
}

#[test]
fn it_expands_entity_ref_filter() {
    let prop_def_id = Uuid::new_v4();
    let entity_id = "macro|user@test.com".to_string();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![],
            entity_ids: vec![entity_id.clone()],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "pd": prop_def_id,
            "et": "TASK",
            "v": { "er": entity_id }
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn it_ands_multiple_property_filters() {
    let status_id = Uuid::new_v4();
    let priority_id = Uuid::new_v4();
    let option_a = Uuid::new_v4();
    let option_b = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![
            PropertyFilter {
                property_definition_id: status_id.to_string(),
                entity_type: Some("TASK".to_string()),
                option_ids: vec![option_a.to_string()],
                entity_ids: vec![],
            },
            PropertyFilter {
                property_definition_id: priority_id.to_string(),
                entity_type: Some("TASK".to_string()),
                option_ids: vec![option_b.to_string()],
                entity_ids: vec![],
            },
        ],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // Two property filters should be AND'd together
    assert!(
        json.get("&").is_some(),
        "multiple property filters should AND together"
    );
}

#[test]
fn it_ors_mixed_option_and_entity_ref_within_single_filter() {
    let prop_def_id = Uuid::new_v4();
    let option_id = Uuid::new_v4();
    let entity_id = "some-entity-id".to_string();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![option_id.to_string()],
            entity_ids: vec![entity_id],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // option_id and entity_id should be OR'd within the same filter
    assert!(
        json.get("|").is_some(),
        "option_ids and entity_ids within one filter should OR together"
    );
}

#[test]
fn empty_property_filters_produce_no_ast() {
    let f = EntityFilters {
        property_filters: vec![],
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).unwrap().is_none(),
        "empty property_filters should produce no AST"
    );
}

#[test]
fn property_filter_with_empty_values_produce_no_ast() {
    let prop_def_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).unwrap().is_none(),
        "property filter with no values should produce no AST"
    );
}

#[test]
fn it_expands_property_filter_without_entity_type() {
    let prop_def_id = Uuid::new_v4();
    let option_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: None,
            option_ids: vec![option_id.to_string()],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "pd": prop_def_id,
            "v": { "so": option_id }
        }
    });

    // entity_type should be absent in serialization when None
    assert_eq!(json, exp);
}

#[test]
fn it_expands_property_filter_with_entity_type() {
    let prop_def_id = Uuid::new_v4();
    let option_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![option_id.to_string()],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    let ast_result = EntityFilterAst::new_from_filters(f).unwrap().unwrap();
    let ast = Arc::into_inner(ast_result.properties_filter.unwrap()).unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "pd": prop_def_id,
            "et": "TASK",
            "v": { "so": option_id }
        }
    });

    // entity_type should be present when Some
    assert_eq!(json, exp);
}

#[test]
fn it_expands_single_document_sub_type() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            sub_types: vec!["task".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "dst": "task"
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn it_expands_sub_type_with_file_type() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            file_types: vec!["pdf".to_string()],
            sub_types: vec!["task".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .document_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    // Should be AND of file_type and sub_type
    assert!(
        json.get("&").is_some(),
        "file_type and sub_type should AND together"
    );
    let as_text = serde_json::to_string(&json).unwrap();
    assert!(as_text.contains(r#""dst":"task""#));
    assert!(as_text.contains(r#""ft":"pdf""#));
}

#[test]
fn invalid_sub_type_returns_error() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            sub_types: vec!["nonexistent".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).is_err(),
        "invalid sub type should return an error"
    );
}

#[test]
fn empty_sub_types_produce_no_ast() {
    let f = EntityFilters {
        document_filters: DocumentFilters {
            sub_types: vec![],
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).unwrap().is_none(),
        "empty sub_types should produce no AST"
    );
}

#[test]
fn invalid_entity_type_returns_error() {
    let prop_def_id = Uuid::new_v4();
    let option_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK'; DROP TABLE documents; --".to_string()),
            option_ids: vec![option_id.to_string()],
            entity_ids: vec![],
        }],
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).is_err(),
        "SQL injection in entity_type should be rejected"
    );
}

#[test]
fn entity_ref_with_single_quote_returns_error() {
    let prop_def_id = Uuid::new_v4();
    let f = EntityFilters {
        property_filters: vec![PropertyFilter {
            property_definition_id: prop_def_id.to_string(),
            entity_type: Some("TASK".to_string()),
            option_ids: vec![],
            entity_ids: vec!["x'); DROP TABLE documents; --".to_string()],
        }],
        ..Default::default()
    };

    assert!(
        EntityFilterAst::new_from_filters(f).is_err(),
        "SQL injection in entity_ids should be rejected"
    );
}

#[test]
fn entity_ref_ast_deserialization_rejects_injection() {
    let json = serde_json::json!({
        "l": {
            "pd": Uuid::new_v4(),
            "et": "TASK",
            "v": { "er": "x'); DROP TABLE documents; --" }
        }
    });

    let result = serde_json::from_value::<filter_ast::Expr<properties::PropertiesLiteral>>(json);
    assert!(
        result.is_err(),
        "direct AST deserialization should reject SQL injection in EntityRef"
    );
}

#[test]
fn entity_type_ast_deserialization_rejects_invalid() {
    let json = serde_json::json!({
        "l": {
            "pd": Uuid::new_v4(),
            "et": "INVALID_TYPE",
            "v": { "so": Uuid::new_v4() }
        }
    });

    let result = serde_json::from_value::<filter_ast::Expr<properties::PropertiesLiteral>>(json);
    assert!(
        result.is_err(),
        "direct AST deserialization should reject invalid entity type"
    );
}

#[test]
fn call_filter_status_expands_status_only() {
    let f = CallFilters {
        status: Some(CallStatus::Missed),
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f)
        .unwrap()
        .expect("status filter should expand to a literal");
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({ "l": { "Status": "MISSED" } });
    assert_eq!(json, exp);
}

#[test]
fn call_filter_status_expands_channel_and_status_as_and() {
    let channel_id = Uuid::new_v4();
    let f = CallFilters {
        channel_ids: vec![channel_id.to_string()],
        status: Some(CallStatus::Unattended),
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f).unwrap().unwrap();
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({
        "&": [
            { "l": { "ChannelId": channel_id } },
            { "l": { "Status": "UNATTENDED" } }
        ]
    });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_call_filter_with_attended_only() {
    let f = CallFilters {
        attended: Some(true),
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f)
        .unwrap()
        .expect("attended filter should expand to a literal");
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({ "l": { "Attended": true } });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_call_filter_with_attended_false() {
    let f = CallFilters {
        attended: Some(false),
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f).unwrap().unwrap();
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({ "l": { "Attended": false } });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_call_filter_with_channel_and_attended_as_and() {
    let channel_id = Uuid::new_v4();
    let f = CallFilters {
        channel_ids: vec![channel_id.to_string()],
        attended: Some(true),
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f).unwrap().unwrap();
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({
        "&": [
            { "l": { "ChannelId": channel_id } },
            { "l": { "Attended": true } }
        ]
    });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_call_filter_with_call_ids() {
    let call_id = Uuid::new_v4();
    let f = CallFilters {
        call_ids: vec![call_id.to_string()],
        ..Default::default()
    };
    let ast = CallFilters::expand_ast(f).unwrap().unwrap();
    let json = serde_json::to_value(&ast).unwrap();
    let exp = json!({ "l": { "CallId": call_id } });
    assert_eq!(json, exp);
}

#[test]
fn it_expands_call_filter_without_attended_is_none_when_empty() {
    let f = CallFilters::default();
    let ast = CallFilters::expand_ast(f).unwrap();
    assert!(ast.is_none(), "empty filter should expand to None");
}

#[test]
fn foreign_entity_empty_filters_produce_no_ast() {
    let ast = ForeignEntityFilters::expand_ast(ForeignEntityFilters::default()).unwrap();
    assert!(ast.is_none(), "empty filter should expand to None");
}

#[test]
fn foreign_entity_ids_parse_as_uuid_literals() {
    let id = Uuid::new_v4();
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            ids: vec![id.to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "l": {
            "id": id
        }
    });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_external_ids_expand_as_or_tree() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            foreign_entity_ids: vec!["external-a".to_string(), "external-b".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "|": [
            { "l": { "feid": "external-a" } },
            { "l": { "feid": "external-b" } }
        ]
    });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_sources_expand_as_or_tree() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            foreign_entity_sources: vec!["github".to_string(), "linear".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "|": [
            { "l": { "fes": "github" } },
            { "l": { "fes": "linear" } }
        ]
    });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_includes_me_expands_as_me_literal() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            includes_me: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({ "l": "me" });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_includes_me_ands_with_sources() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            foreign_entity_sources: vec!["github_pull_request".to_string()],
            includes_me: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "&": [
            { "l": { "fes": "github_pull_request" } },
            { "l": "me" }
        ]
    });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_notification_filters_expand_as_literals() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            notification_filters: crate::NotificationFilters {
                states: vec![
                    crate::NotificationState::Done,
                    crate::NotificationState::Unseen,
                ],
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_string(&ast).unwrap();
    assert!(
        json.contains(r#""ns":"done""#),
        "expected done state literal: {json}"
    );
    assert!(
        json.contains(r#""ns":"unseen""#),
        "expected unseen state literal: {json}"
    );
}

#[test]
fn foreign_entity_notification_done_ands_with_source() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            foreign_entity_sources: vec!["github_pull_request".to_string()],
            notification_filters: crate::NotificationFilters {
                states: vec![crate::NotificationState::Done],
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let ast = Arc::into_inner(
        EntityFilterAst::new_from_filters(f)
            .unwrap()
            .unwrap()
            .foreign_entity_filter
            .unwrap(),
    )
    .unwrap();

    let json = serde_json::to_value(ast).unwrap();
    let exp = json!({
        "&": [
            { "l": { "fes": "github_pull_request" } },
            { "l": { "ns": "done" } }
        ]
    });

    assert_eq!(json, exp);
}

#[test]
fn foreign_entity_invalid_id_returns_uuid_error() {
    let f = EntityFilters {
        foreign_entity_filters: ForeignEntityFilters {
            ids: vec!["not-a-uuid".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let err = EntityFilterAst::new_from_filters(f).unwrap_err();
    assert_matches!(err, ExpandErr::Uuid(_));
}

#[test]
fn properties_filter_can_apply_to_respects_literal_entity_types() {
    use properties::{
        PropertiesLiteral, PropertyEntityType, PropertyMatchValue, properties_filter_can_apply_to,
    };
    let lit = |entity_type| {
        Expr::Literal(PropertiesLiteral {
            property_definition_id: Uuid::new_v4(),
            entity_type,
            value: PropertyMatchValue::SelectOption(Uuid::new_v4()),
        })
    };

    let untyped = lit(None);
    assert!(properties_filter_can_apply_to(
        &untyped,
        &[PropertyEntityType::Thread]
    ));

    let task_only = lit(Some(PropertyEntityType::Task));
    assert!(!properties_filter_can_apply_to(
        &task_only,
        &[PropertyEntityType::Thread]
    ));

    let either = Expr::or(lit(Some(PropertyEntityType::Task)), lit(None));
    assert!(properties_filter_can_apply_to(
        &either,
        &[PropertyEntityType::Thread]
    ));

    let negated = Expr::is_not(lit(Some(PropertyEntityType::Task)));
    assert!(properties_filter_can_apply_to(
        &negated,
        &[PropertyEntityType::Thread]
    ));
}

#[test]
fn properties_filter_matches_propertyless_evaluates_literals_as_false() {
    use properties::{
        PropertiesLiteral, PropertyMatchValue, properties_filter_matches_propertyless,
    };
    let lit = || {
        Expr::Literal(PropertiesLiteral {
            property_definition_id: Uuid::new_v4(),
            entity_type: None,
            value: PropertyMatchValue::SelectOption(Uuid::new_v4()),
        })
    };

    assert!(!properties_filter_matches_propertyless(&lit()));
    assert!(!properties_filter_matches_propertyless(&Expr::or(
        lit(),
        lit()
    )));
    assert!(properties_filter_matches_propertyless(&Expr::is_not(lit())));
    assert!(!properties_filter_matches_propertyless(&Expr::and(
        lit(),
        Expr::is_not(lit())
    )));
}

#[test]
fn initiatives_expand_with_properties_without_enabling_legacy_queries() {
    let empty = EntityFilterAst::new_from_filters(EntityFilters::default()).unwrap();
    assert!(empty.is_none());
    let filter = EntityFilters {
        initiative_filters: crate::InitiativeFilters {
            include: true,
            name: Some("Launch".into()),
            ..Default::default()
        },
        property_filters: vec![crate::PropertyFilter {
            property_definition_id: "00000001-0000-0000-0000-000000000002".into(),
            entity_type: Some("INITIATIVE".into()),
            option_ids: vec!["00000000-0000-0000-0000-000000000001".into()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let ast = EntityFilterAst::new_from_filters(filter).unwrap().unwrap();
    assert!(ast.initiative_filter.is_some());
    assert!(ast.properties_filter.is_some());
}

#[test]
fn database_rows_are_requested_by_naming_a_table_or_a_row_outside_a_not() {
    use database_row::{DatabaseRowLiteral, database_rows_requested};
    let table = Uuid::from_u128(0x7ab1e);
    let row = Uuid::from_u128(0x0001);

    assert!(!database_rows_requested(None));
    assert!(database_rows_requested(Some(&Expr::val(
        DatabaseRowLiteral::TableId(table)
    ))));
    assert!(database_rows_requested(Some(&Expr::or(
        Expr::val(DatabaseRowLiteral::TableId(table)),
        Expr::val(DatabaseRowLiteral::Id(row)),
    ))));
    assert!(!database_rows_requested(Some(&Expr::is_not(Expr::val(
        DatabaseRowLiteral::TableId(table)
    )))));
    assert!(database_rows_requested(Some(&Expr::and(
        Expr::val(DatabaseRowLiteral::TableId(table)),
        Expr::is_not(Expr::val(DatabaseRowLiteral::Id(row))),
    ))));

    assert_eq!(
        serde_json::to_value(EntityFilterAst {
            database_row_filter: Some(Arc::new(Expr::val(DatabaseRowLiteral::TableId(table)))),
            ..EntityFilterAst::default()
        })
        .unwrap()["drf"],
        json!({ "l": { "t": "00000000-0000-0000-0000-00000007ab1e" } })
    );
}
