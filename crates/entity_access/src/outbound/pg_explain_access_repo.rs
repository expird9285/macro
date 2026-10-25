use crate::domain::{
    models::{AccessError, AccessGrant, EntityType, ForeignEntityAuthEntity, UserTeamInfo},
    ports::ExplainAccessRepository,
};
use crate::outbound::pg_access_repo::queries;
use macro_user_id::{lowercased::Lowercase, user_id::MacroUserId};
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod test;

/// PostgreSQL-backed implementation of [`ExplainAccessRepository`].
#[derive(Clone)]
pub struct PgExplainAccessRepository {
    pool: PgPool,
}

impl PgExplainAccessRepository {
    /// Create a new explain repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn foreign_entity_source_pairs(
    user_id: &MacroUserId<Lowercase<'_>>,
    user_team: Option<UserTeamInfo>,
) -> (Vec<String>, Vec<String>) {
    let mut source_ids = vec![user_id.as_ref().to_string()];
    let mut source_auth_entities = vec![ForeignEntityAuthEntity::User.as_str().to_string()];

    if let Some(user_team) = user_team {
        source_ids.push(user_team.team_id.to_string());
        source_auth_entities.push(ForeignEntityAuthEntity::Team.as_str().to_string());
    }

    (source_ids, source_auth_entities)
}

fn parse_uuid(entity_id: &str, message: &'static str) -> Result<Uuid, AccessError> {
    Uuid::parse_str(entity_id).map_err(|_| AccessError::BadRequest(message))
}

impl ExplainAccessRepository for PgExplainAccessRepository {
    #[tracing::instrument(err, skip(self))]
    async fn list_access_grants(
        &self,
        user_id: &MacroUserId<Lowercase<'_>>,
        entity_id: &str,
        entity_type: EntityType,
    ) -> Result<Vec<AccessGrant>, AccessError> {
        match entity_type {
            EntityType::Document => {
                let document_id = parse_uuid(entity_id, "Invalid document ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::document_access::explain_document_access(
                    &self.pool,
                    &document_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Chat => {
                let chat_id = parse_uuid(entity_id, "Invalid chat ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(
                    queries::chat_access::explain_chat_access(&self.pool, &chat_id, &source_ids)
                        .await?,
                )
            }
            EntityType::Project => {
                let project_id = parse_uuid(entity_id, "Invalid project ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::project_access::explain_project_access(
                    &self.pool,
                    &project_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Call => {
                let call_id = parse_uuid(entity_id, "Invalid call ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(
                    queries::call_access::explain_call_access(&self.pool, &call_id, &source_ids)
                        .await?,
                )
            }
            EntityType::AgentSession => {
                let session_id = parse_uuid(entity_id, "Invalid agent session ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::agent_session_access::explain_agent_session_access(
                    &self.pool,
                    &session_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Database => {
                let database_id = parse_uuid(entity_id, "Invalid database ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::database_access::explain_database_access(
                    &self.pool,
                    &database_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Form => {
                let form_id = parse_uuid(entity_id, "Invalid form ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(
                    queries::form_access::explain_form_access(&self.pool, &form_id, &source_ids)
                        .await?,
                )
            }
            EntityType::DatabaseRow => {
                let row_id = parse_uuid(entity_id, "Invalid database row ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::database_row_access::explain_database_row_access(
                    &self.pool,
                    &row_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Initiative => {
                let initiative_id = parse_uuid(entity_id, "Invalid initiative ID format")?;
                let source_ids = queries::get_user_source_ids(&self.pool, Some(user_id))
                    .await
                    .map_err(anyhow_access_error)?;
                Ok(queries::initiative_access::explain_initiative_access(
                    &self.pool,
                    &initiative_id,
                    &source_ids,
                )
                .await?)
            }
            EntityType::Channel => {
                let channel_id = parse_uuid(entity_id, "Invalid channel ID format")?;
                Ok(queries::channel_role::explain_channel_access(
                    &self.pool,
                    &channel_id,
                    user_id.as_ref(),
                )
                .await?)
            }
            EntityType::Team => {
                let team_id = parse_uuid(entity_id, "Invalid team ID format")?;
                Ok(queries::team_access::get_user_team(&self.pool, user_id)
                    .await?
                    .filter(|team| team.team_id == team_id)
                    .map(|team| AccessGrant::TeamMembership { role: team.role })
                    .into_iter()
                    .collect())
            }
            EntityType::ForeignEntity => {
                let foreign_entity_id = parse_uuid(entity_id, "Invalid foreign entity ID format")?;
                let user_team = queries::team_access::get_user_team(&self.pool, user_id).await?;
                let (source_ids, source_auth_entities) =
                    foreign_entity_source_pairs(user_id, user_team);
                Ok(queries::foreign_entity_access::list_foreign_entity_grants(
                    &self.pool,
                    &foreign_entity_id,
                    &source_ids,
                    &source_auth_entities,
                )
                .await?)
            }
            EntityType::StaticFile => Ok(vec![AccessGrant::StaticFileAlwaysView]),
            // Removed features keep their stored entity types but grant nothing.
            EntityType::User
            | EntityType::ChannelMessage
            | EntityType::Skill
            | EntityType::ScheduledAction
            | EntityType::EmailThread
            | EntityType::CalendarEvent
            | EntityType::CrmCompany
            | EntityType::CrmContact
            | EntityType::CrmPipeline
            | EntityType::Reminder => Ok(vec![]),
        }
    }
}

fn anyhow_access_error(e: anyhow::Error) -> AccessError {
    match e.downcast::<sqlx::Error>() {
        Ok(sqlx_error) => AccessError::from(sqlx_error),
        Err(other) => AccessError::Internal(rootcause::report!(other).into_dynamic()),
    }
}
