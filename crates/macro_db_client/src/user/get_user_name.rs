#[cfg(test)]
mod tests;

use macro_user_id::lowercased::Lowercase;
use macro_user_id::user_id::MacroUserId;
use model::user::UserName;
use non_empty::NonEmpty;

#[tracing::instrument(skip(db))]
pub async fn get_user_name(db: &sqlx::PgPool, macro_user_id: &str) -> anyhow::Result<UserName> {
    let macro_user_id = macro_uuid::string_to_uuid(macro_user_id)?;
    let name: Option<UserName> = sqlx::query!(
        r#"
            SELECT macro_user_id, first_name, last_name FROM macro_user_info WHERE macro_user_id = $1
        "#,
        &macro_user_id
    )
    .map(|row| UserName {
        id: row.macro_user_id.to_string(), // TODO: this may mess up FE?
        first_name: row.first_name,
        last_name: row.last_name,
    })
    .fetch_optional(db)
    .await?;

    // The row is created lazily by the first name write (put_name upserts),
    // so a user who never set their name has none — that's "no name yet",
    // not an error.
    Ok(name.unwrap_or_else(|| UserName {
        id: macro_user_id.to_string(),
        first_name: None,
        last_name: None,
    }))
}

#[tracing::instrument(skip(db))]
pub async fn get_user_names(
    db: &sqlx::PgPool,
    user_profile_ids: &Vec<String>,
) -> anyhow::Result<Vec<UserName>> {
    let user_names = sqlx::query!(
        r#"
            SELECT 
                u.id as user_profile_id, 
                mui.first_name, 
                mui.last_name
            FROM macro_user_info mui
            JOIN "User" u ON mui.macro_user_id = u.macro_user_id
            WHERE u.id = ANY($1)
        "#,
        user_profile_ids
    )
    .map(|row| UserName {
        id: row.user_profile_id,
        first_name: row.first_name,
        last_name: row.last_name,
    })
    .fetch_all(db)
    .await?;

    Ok(user_names)
}

/// Gets macro user names for the passed user profile ids. A name stored as the
/// `N/A` placeholder is reported as absent; ids with no `User` row are omitted.
#[tracing::instrument(skip(db), err)]
pub async fn get_user_names_with_email(
    db: &sqlx::PgPool,
    user_profile_ids: NonEmpty<Vec<MacroUserId<Lowercase<'_>>>>,
) -> anyhow::Result<Vec<UserName>> {
    let user_profile_ids_str: Vec<&str> = user_profile_ids.iter().map(|id| id.as_ref()).collect();

    let user_names = sqlx::query!(
        r#"
        WITH requested_ids AS (
            SELECT DISTINCT id
            FROM UNNEST($1::text[]) AS requested(id)
        )
        SELECT
            req.id as "user_profile_id!",
            NULLIF(mui.first_name, 'N/A') as "first_name",
            NULLIF(mui.last_name, 'N/A') as "last_name"
        FROM requested_ids req
        JOIN "User" u ON u.id = req.id
        LEFT JOIN macro_user_info mui ON mui.macro_user_id = u.macro_user_id
        "#,
        &user_profile_ids_str as &[&str]
    )
    .map(|row| UserName {
        id: row.user_profile_id,
        first_name: row.first_name,
        last_name: row.last_name,
    })
    .fetch_all(db)
    .await?;

    Ok(user_names)
}
