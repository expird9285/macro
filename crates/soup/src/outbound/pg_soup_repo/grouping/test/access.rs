use super::*;

async fn assert_membership(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let items = expanded_dynamic_cursor_soup_grouped(
        pool,
        GroupedDynamicCursorArgs {
            user_id: MacroUserIdStr::parse_from_str("macro|user-1@test.com")?,
            limit: 2,
            cursor: Query::Sort(SimpleSortMethod::UpdatedAt, EntityFilterAst::mock_empty()),
            exclude_frecency: false,
            grouping: GroupingConfig {
                field: GroupByField::EntityType,
                group_key: None,
                per_group_limit: None,
            },
        },
    )
    .await?
    .collect::<Vec<_>>();
    for (key, expected) in [
        (
            "document",
            vec![
                "11111111-0000-0000-0000-000000000000",
                "11111111-dddd-dddd-dddd-dddddddddddd",
                "11111111-cccc-cccc-cccc-cccccccccccc",
                "11111111-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                "11111111-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            ],
        ),
        (
            "chat",
            vec![
                "22222222-0000-0000-0000-000000000000",
                "22222222-cccc-cccc-cccc-cccccccccccc",
                "22222222-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                "22222222-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            ],
        ),
        (
            "project",
            vec![
                "dddddddd-ffff-ffff-ffff-ffffffffffff",
                "cccccccc-ffff-ffff-ffff-ffffffffffff",
                "bbbbbbbb-ffff-ffff-ffff-ffffffffffff",
                "aaaaaaaa-ffff-ffff-ffff-ffffffffffff",
            ],
        ),
    ] {
        let group = items
            .iter()
            .filter(|item| item.key == key)
            .collect::<Vec<_>>();
        assert_eq!(
            group
                .iter()
                .map(|i| i.item.id().to_string())
                .collect::<Vec<_>>(),
            expected
        );
        for (index, item) in group.iter().enumerate() {
            assert_eq!(item.total_group_count, expected.len());
            assert_eq!(item.index_in_group, index + 1);
        }
    }
    assert_eq!(items.len(), 13);
    Ok(())
}

#[sqlx::test(
    fixtures(
        path = "../../../../../../macro_db_client/fixtures",
        scripts("mixed_items_expanded")
    ),
    migrator = "MACRO_DB_MIGRATIONS"
)]
async fn inherited_and_direct_grants_without_owner_fallback(
    pool: Pool<Postgres>,
) -> anyhow::Result<()> {
    assert_membership(&pool).await
}

#[sqlx::test(
    fixtures(
        path = "../../../../../../macro_db_client/fixtures",
        scripts("mixed_items_expanded")
    ),
    fixtures(path = ".", scripts("access")),
    migrator = "MACRO_DB_MIGRATIONS"
)]
async fn source_membership_deduplicates_grants(pool: Pool<Postgres>) -> anyhow::Result<()> {
    assert_membership(&pool).await?;
    sqlx::query!(r#"UPDATE "Document" SET "deletedAt" = now() WHERE id = '11111111-0000-0000-0000-000000000000'"#).execute(&pool).await?;
    let items = expanded_dynamic_cursor_soup_grouped(
        &pool,
        GroupedDynamicCursorArgs {
            user_id: MacroUserIdStr::parse_from_str("macro|user-1@test.com")?,
            limit: 50,
            cursor: Query::Sort(
                SimpleSortMethod::UpdatedAt,
                super::pagination::task_filter(),
            ),
            exclude_frecency: false,
            grouping: GroupingConfig {
                field: GroupByField::EntityType,
                group_key: None,
                per_group_limit: None,
            },
        },
    )
    .await?
    .collect::<Vec<_>>();
    assert!(
        items.is_empty(),
        "deleted task must be excluded despite multiple grants"
    );
    Ok(())
}
