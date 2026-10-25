//! Small SQL helpers for `SharePermission` and `ChannelSharePermission` rows.

pub mod team_share;

/// Create private sharing state for an entity, without team defaults or grants.
pub async fn create_private_share_permission<'e, E>(
    executor: E,
    id: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(r#"INSERT INTO "SharePermission" (id, "linkShare", "linkShareAccessLevel") VALUES ($1, NULL, NULL)"#, id)
        .execute(executor).await?;
    Ok(())
}

use models_permissions::share_permission::access_level::AccessLevel;
use models_permissions::share_permission::{LinkShare, TeamLinkShareDefault};
use sqlx::{Executor, Postgres};

#[cfg(test)]
mod test;

/// Look up the link-share preference of the user's team.
///
/// Returns `None` if the user is not on a team; otherwise the team's preference, where an inner
/// `None` means the team turned link sharing off by default. If a user somehow belongs to
/// multiple teams the strongest membership wins (`team_role` orders `member < admin < owner`),
/// mirroring `entity_access`'s `get_user_team`.
pub async fn get_team_default_link_share<'e, E>(
    executor: E,
    user_id: &str,
) -> Result<Option<TeamLinkShareDefault>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query!(
        r#"
        SELECT t.default_link_share as "default_link_share: LinkShare"
        FROM team_user tu
        JOIN team t ON t.id = tu.team_id
        WHERE tu.user_id = $1
        ORDER BY tu.team_role DESC
        LIMIT 1
        "#,
        user_id,
    )
    .fetch_optional(executor)
    .await?;

    Ok(row.map(|r| TeamLinkShareDefault(r.default_link_share)))
}

/// The result of attempting to insert a channel share permission row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertChannelSharePermissionResult {
    /// A new row was inserted.
    Inserted,
    /// A row for the same share permission and channel already existed.
    AlreadyExists,
}

/// Look up the share permission ID for a supported shareable item.
pub async fn get_share_permission_id<'e, E>(
    executor: E,
    item_id: &str,
    item_type: &str,
) -> anyhow::Result<String>
where
    E: Executor<'e, Database = Postgres>,
{
    let share_permission_id = match item_type {
        "document" => {
            sqlx::query_scalar!(
                r#"
                SELECT dp."sharePermissionId" as "share_permission_id!"
                FROM "DocumentPermission" dp
                WHERE dp."documentId" = $1
                "#,
                item_id,
            )
            .fetch_one(executor)
            .await?
        }
        "chat" => {
            sqlx::query_scalar!(
                r#"
                SELECT cp."sharePermissionId" as "share_permission_id!"
                FROM "ChatPermission" cp
                WHERE cp."chatId" = $1
                "#,
                item_id,
            )
            .fetch_one(executor)
            .await?
        }
        "project" => {
            sqlx::query_scalar!(
                r#"
                SELECT pp."sharePermissionId" as "share_permission_id!"
                FROM "ProjectPermission" pp
                WHERE pp."projectId" = $1
                "#,
                item_id,
            )
            .fetch_one(executor)
            .await?
        }
        "call" => {
            let item_id = macro_uuid::string_to_uuid(item_id)?;
            sqlx::query_scalar!(
                r#"
                SELECT share_permission_id as "share_permission_id!"
                FROM call_entities WHERE id = $1
                "#,
                item_id,
            )
            .fetch_one(executor)
            .await?
        }
        "initiative" => {
            let item_id = macro_uuid::string_to_uuid(item_id)?;
            sqlx::query_scalar!(
                r#"
                SELECT share_permission_id as "share_permission_id!"
                FROM initiative WHERE id = $1::uuid
                "#,
                item_id,
            )
            .fetch_one(executor)
            .await?
        }
        _ => anyhow::bail!("unsupported item type {item_type}"),
    };

    Ok(share_permission_id)
}

/// Insert a channel share permission row without mutating an existing row.
pub async fn insert_channel_share_permission<'e, E>(
    executor: E,
    share_permission_id: &str,
    channel_id: &str,
    access_level: AccessLevel,
) -> Result<InsertChannelSharePermissionResult, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query!(
        r#"
        INSERT INTO "ChannelSharePermission" ("share_permission_id", "channel_id", "access_level")
        VALUES ($1, $2, $3)
        ON CONFLICT ("share_permission_id", "channel_id") DO NOTHING
        "#,
        share_permission_id,
        channel_id,
        access_level as _,
    )
    .execute(executor)
    .await?;

    if result.rows_affected() == 0 {
        Ok(InsertChannelSharePermissionResult::AlreadyExists)
    } else {
        Ok(InsertChannelSharePermissionResult::Inserted)
    }
}
