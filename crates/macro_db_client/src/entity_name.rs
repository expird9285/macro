use models_opensearch::SearchEntityType;

/// Gets the entity name and macro user id of the owner for the provided
/// entity id and entity type
#[tracing::instrument(skip(db), err)]
pub async fn get_entity_name_and_owner(
    db: &sqlx::PgPool,
    entity_id: &uuid::Uuid,
    entity_type: &SearchEntityType,
) -> anyhow::Result<(Option<String>, String)> {
    let result: (Option<String>, String) = match entity_type {
        SearchEntityType::Chats => {
            sqlx::query!(
                r#"
                SELECT
                    c.name,
                    c."userId" as user_id
                FROM
                    "Chat" c
                WHERE
                    c.id = $1
                "#,
                &entity_id.to_string(),
            )
            .map(|row| (Some(row.name), row.user_id))
            .fetch_one(db)
            .await?
        }
        SearchEntityType::Documents => {
            sqlx::query!(
                r#"
                SELECT
                    d.name,
                    d.owner
                FROM
                    "Document" d
                WHERE
                    d.id = $1
                "#,
                &entity_id.to_string(),
            )
            .map(|row| (Some(row.name), row.owner))
            .fetch_one(db)
            .await?
        }
        SearchEntityType::Channels => {
            sqlx::query!(
                r#"
                SELECT
                    c.name as "name?",
                    c.owner_id
                FROM
                    "comms_channels" c
                WHERE
                    c.id = $1
                "#,
                entity_id,
            )
            .map(|row| (row.name, row.owner_id))
            .fetch_one(db)
            .await?
        }
        SearchEntityType::AgentSessions | SearchEntityType::Projects => {
            anyhow::bail!("entity type not supported");
        }
        SearchEntityType::CallRecords => {
            anyhow::bail!("entity type not supported");
        }
    };

    Ok(result)
}

#[cfg(test)]
mod test;
