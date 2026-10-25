use super::PgAccessRepository;
use crate::domain::{
    models::{
        AccessError, AccessLevel, AgentSessionParent, BotId, ChannelRoleResult, EntityType,
        ParticipantRole,
    },
    ports::AccessRepository,
};
use macro_db_migrator::MACRO_DB_MIGRATIONS;
use macro_user_id::user_id::MacroUserIdStr;
use sqlx::PgPool;
use uuid::Uuid;

const TEAM_ALPHA: &str = "00000000-0000-0000-0000-0000000ea001";
const TEAM_BETA: &str = "00000000-0000-0000-0000-0000000ea002";
const TEAM_MEMBER: &str = "macro|member@team.com";
const USER_WITHOUT_TEAM: &str = "macro|noteam@team.com";

mod agent_session_parent;
mod channel_call_chat;

fn user_id(value: &str) -> MacroUserIdStr<'static> {
    MacroUserIdStr::try_from(value.to_string()).unwrap()
}

async fn insert_foreign_entity(
    pool: &PgPool,
    id: Uuid,
    stored_for_id: &str,
    stored_for_auth_entity: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO foreign_entity (
            id,
            foreign_entity_id,
            foreign_entity_source,
            metadata,
            stored_for_id,
            stored_for_auth_entity
        )
        VALUES ($1, $2, $3, '{}'::jsonb, $4, $5)
        "#,
        id,
        format!("external-{id}"),
        "test-source",
        stored_for_id,
        stored_for_auth_entity,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn grants_direct_user_access(pool: PgPool) -> anyhow::Result<()> {
    let foreign_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, foreign_entity_id, USER_WITHOUT_TEAM, "user").await?;

    let repo = PgAccessRepository::new(pool);
    let user_id = user_id(USER_WITHOUT_TEAM);

    let has_access = repo
        .has_foreign_entity_access(&foreign_entity_id.to_string(), Some(&user_id))
        .await?;

    assert!(has_access);
    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn grants_team_access(pool: PgPool) -> anyhow::Result<()> {
    let foreign_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, foreign_entity_id, TEAM_ALPHA, "team").await?;

    let repo = PgAccessRepository::new(pool);
    let user_id = user_id(TEAM_MEMBER);

    let has_access = repo
        .has_foreign_entity_access(&foreign_entity_id.to_string(), Some(&user_id))
        .await?;

    assert!(has_access);
    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn denies_unrelated_user_and_team_access(pool: PgPool) -> anyhow::Result<()> {
    let unrelated_user_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, unrelated_user_entity_id, USER_WITHOUT_TEAM, "user").await?;

    let unrelated_team_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, unrelated_team_entity_id, TEAM_BETA, "team").await?;

    let repo = PgAccessRepository::new(pool);
    let user_id = user_id(TEAM_MEMBER);

    let user_access = repo
        .has_foreign_entity_access(&unrelated_user_entity_id.to_string(), Some(&user_id))
        .await?;
    let team_access = repo
        .has_foreign_entity_access(&unrelated_team_entity_id.to_string(), Some(&user_id))
        .await?;

    assert!(!user_access);
    assert!(!team_access);
    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn denies_unauthenticated_access(pool: PgPool) -> anyhow::Result<()> {
    let foreign_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, foreign_entity_id, USER_WITHOUT_TEAM, "user").await?;

    let repo = PgAccessRepository::new(pool);

    let has_access = repo
        .has_foreign_entity_access(&foreign_entity_id.to_string(), None)
        .await?;

    assert!(!has_access);
    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn denies_auth_namespace_mismatch(pool: PgPool) -> anyhow::Result<()> {
    let foreign_entity_id = Uuid::new_v4();
    insert_foreign_entity(&pool, foreign_entity_id, USER_WITHOUT_TEAM, "team").await?;

    let repo = PgAccessRepository::new(pool);
    let user_id = user_id(USER_WITHOUT_TEAM);

    let has_access = repo
        .has_foreign_entity_access(&foreign_entity_id.to_string(), Some(&user_id))
        .await?;

    assert!(!has_access);
    Ok(())
}

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("user_team"))
)]
async fn rejects_invalid_uuid(pool: PgPool) -> anyhow::Result<()> {
    let repo = PgAccessRepository::new(pool);
    let user_id = user_id(USER_WITHOUT_TEAM);

    let error = repo
        .has_foreign_entity_access("not-a-uuid", Some(&user_id))
        .await
        .expect_err("invalid UUID should be rejected");

    assert!(matches!(
        error,
        AccessError::BadRequest("Invalid foreign entity ID format")
    ));
    Ok(())
}

const PG_BOT_OWNER: &str = "macro|pg-bot-owner@example.com";
const PG_OTHER_DOCUMENT_OWNER: &str = "macro|other-document-owner@example.com";

async fn insert_pg_user(pool: &PgPool, user_id: &str, email: &str) -> anyhow::Result<()> {
    let macro_user_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO macro_user (id, username, email, stripe_customer_id)
        VALUES ($1, $2, $3, $2)
        "#,
        macro_user_id,
        user_id,
        email,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO "User" (id, email, macro_user_id)
        VALUES ($1, $2, $3)
        "#,
        user_id,
        email,
        macro_user_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_pg_team(
    pool: &PgPool,
    team_id: Uuid,
    owner_id: &str,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO team (id, name, owner_id) VALUES ($1, $2, $3)",
        team_id,
        name,
        owner_id,
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        r#"
        INSERT INTO team_user (user_id, team_id, team_role)
        VALUES ($1, $2, 'owner')
        "#,
        owner_id,
        team_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_pg_bot_team(pool: &PgPool, team_id: Uuid) -> anyhow::Result<()> {
    insert_pg_user(pool, PG_BOT_OWNER, "pg-bot-owner@example.com").await?;
    insert_pg_team(pool, team_id, PG_BOT_OWNER, "PG Bot Team").await
}

async fn insert_pg_bot(
    pool: &PgPool,
    bot_id: BotId,
    owner_user_id: Option<&str>,
    team_id: Option<Uuid>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO bots (id, kind, owner_user_id, team_id, name, handle)
        VALUES ($1, 'owned', $2, $3, 'PG Bot', $4)
        "#,
        bot_id.as_uuid(),
        owner_user_id,
        team_id,
        format!("pg-bot-{bot_id}"),
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_pg_bot_channel(
    pool: &PgPool,
    channel_id: Uuid,
    channel_type: &str,
    team_id: Option<Uuid>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO comms_channels (id, name, channel_type, owner_id, team_id)
        VALUES ($1, 'PG Bot Channel', $2::text::comms_channel_type, $3, $4)
        "#,
        channel_id,
        channel_type,
        PG_BOT_OWNER,
        team_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_pg_bot_participant(
    pool: &PgPool,
    channel_id: Uuid,
    bot_id: BotId,
    role: &str,
) -> anyhow::Result<()> {
    let principal = bot_id.into_storage_id();
    sqlx::query!(
        r#"
        INSERT INTO comms_channel_participants (channel_id, role, user_id)
        VALUES ($1, $2::text::comms_participant_role, $3)
        "#,
        channel_id,
        role,
        principal.as_ref(),
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_pg_bot_entity_access(
    pool: &PgPool,
    entity_id: Uuid,
    entity_type: &str,
    source_id: &str,
    source_type: &str,
    access_level: AccessLevel,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO entity_access (
            entity_id,
            entity_type,
            source_id,
            source_type,
            access_level
        )
        VALUES (
            $1,
            $2,
            $3,
            $4::text::entity_access_source_type,
            $5::text::"AccessLevel"
        )
        "#,
        entity_id,
        entity_type,
        source_id,
        source_type,
        access_level.to_string(),
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_pg_link_shared_document(
    pool: &PgPool,
    document_id: Uuid,
    owner_id: &str,
    link_share: &str,
    access_level: AccessLevel,
) -> anyhow::Result<()> {
    let document_id = document_id.to_string();
    let permission_id = Uuid::new_v4().to_string();
    let access_level = access_level.to_string();

    sqlx::query!(
        r#"INSERT INTO "Document" (id, name, owner) VALUES ($1, 'Link Shared Document', $2)"#,
        document_id,
        owner_id,
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        r#"
        INSERT INTO "SharePermission" (
            id,
            "linkShare",
            "linkShareAccessLevel"
        )
        VALUES ($1, $2, $3::text::"AccessLevel")
        "#,
        permission_id,
        link_share,
        access_level,
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        r#"
        INSERT INTO "DocumentPermission" ("documentId", "sharePermissionId")
        VALUES ($1, $2)
        "#,
        document_id,
        permission_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_includes_team_channels_and_bot_grants(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let bot_principal = bot_id.into_storage_id();
    let private_channel_id = Uuid::new_v4();
    let team_channel_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_bot_channel(&pool, private_channel_id, "private", None).await?;
    insert_pg_bot_channel(&pool, team_channel_id, "team", Some(team_id)).await?;
    insert_pg_bot_participant(&pool, private_channel_id, bot_id, "member").await?;

    let grants = [
        (team_id.to_string(), "team", AccessLevel::View),
        (team_channel_id.to_string(), "channel", AccessLevel::Comment),
        (private_channel_id.to_string(), "channel", AccessLevel::Edit),
        (bot_principal.to_string(), "user", AccessLevel::Owner),
    ];
    let repo = PgAccessRepository::new(pool);

    for (source_id, source_type, expected) in grants {
        let document_id = Uuid::new_v4();
        insert_pg_bot_entity_access(
            &repo.pool,
            document_id,
            "document",
            &source_id,
            source_type,
            expected,
        )
        .await?;

        let access = repo
            .get_team_entity_access(
                bot_id,
                team_id,
                &document_id.to_string(),
                EntityType::Document,
            )
            .await?;
        assert_eq!(access, Some(expected), "source {source_id}");
    }
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_resolves_bot_typed_grant(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let bot_principal = bot_id.into_storage_id();
    let document_id = Uuid::new_v4();

    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_bot_entity_access(
        &pool,
        document_id,
        "document",
        bot_principal.as_ref(),
        "bot",
        AccessLevel::Owner,
    )
    .await?;

    let access = PgAccessRepository::new(pool)
        .get_team_entity_access(
            bot_id,
            team_id,
            &document_id.to_string(),
            EntityType::Document,
        )
        .await?;

    assert_eq!(access, Some(AccessLevel::Owner));
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_allows_public_link_sharing(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let document_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_link_shared_document(
        &pool,
        document_id,
        PG_BOT_OWNER,
        "PUBLIC",
        AccessLevel::Comment,
    )
    .await?;

    let access = PgAccessRepository::new(pool)
        .get_team_entity_access(
            bot_id,
            team_id,
            &document_id.to_string(),
            EntityType::Document,
        )
        .await?;

    assert_eq!(access, Some(AccessLevel::Comment));
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_allows_team_link_for_document_owner_team(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let document_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_link_shared_document(&pool, document_id, PG_BOT_OWNER, "TEAM", AccessLevel::Edit)
        .await?;

    let access = PgAccessRepository::new(pool)
        .get_team_entity_access(
            bot_id,
            team_id,
            &document_id.to_string(),
            EntityType::Document,
        )
        .await?;

    assert_eq!(access, Some(AccessLevel::Edit));
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_denies_team_link_for_another_document_owner_team(
    pool: PgPool,
) -> anyhow::Result<()> {
    let bot_team_id = Uuid::new_v4();
    let owner_team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let document_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, bot_team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(bot_team_id)).await?;
    insert_pg_user(
        &pool,
        PG_OTHER_DOCUMENT_OWNER,
        "other-document-owner@example.com",
    )
    .await?;
    insert_pg_team(
        &pool,
        owner_team_id,
        PG_OTHER_DOCUMENT_OWNER,
        "Other Document Owner Team",
    )
    .await?;
    insert_pg_link_shared_document(
        &pool,
        document_id,
        PG_OTHER_DOCUMENT_OWNER,
        "TEAM",
        AccessLevel::Edit,
    )
    .await?;

    let access = PgAccessRepository::new(pool)
        .get_team_entity_access(
            bot_id,
            bot_team_id,
            &document_id.to_string(),
            EntityType::Document,
        )
        .await?;

    assert_eq!(access, None);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_rejects_another_team(pool: PgPool) -> anyhow::Result<()> {
    let owning_team_id = Uuid::new_v4();
    let other_team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let document_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, owning_team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(owning_team_id)).await?;
    insert_pg_bot_entity_access(
        &pool,
        document_id,
        "document",
        &owning_team_id.to_string(),
        "team",
        AccessLevel::Owner,
    )
    .await?;

    let access = PgAccessRepository::new(pool)
        .get_team_entity_access(
            bot_id,
            other_team_id,
            &document_id.to_string(),
            EntityType::Document,
        )
        .await?;

    assert_eq!(access, None);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_channel_role_uses_scoped_channel_rules(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let team_channel_id = Uuid::new_v4();
    let public_channel_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_bot_channel(&pool, team_channel_id, "team", Some(team_id)).await?;
    insert_pg_bot_channel(&pool, public_channel_id, "public", None).await?;
    let repo = PgAccessRepository::new(pool);

    assert_eq!(
        repo.get_team_channel_role(&team_channel_id, team_id, bot_id)
            .await?,
        ChannelRoleResult::ViewOnly,
    );
    assert_eq!(
        repo.get_team_channel_role(&public_channel_id, team_id, bot_id)
            .await?,
        ChannelRoleResult::NoAccess,
    );

    insert_pg_bot_participant(&repo.pool, public_channel_id, bot_id, "admin").await?;
    assert_eq!(
        repo.get_team_channel_role(&public_channel_id, team_id, bot_id)
            .await?,
        ChannelRoleResult::Role(ParticipantRole::Admin),
    );
    Ok(())
}

/// A direct agent conversation between `owner` and `bot_id`, holding just the
/// two of them as opening one does.
async fn insert_pg_direct_agent_conversation(
    pool: &PgPool,
    channel_id: Uuid,
    owner: &str,
    bot_id: BotId,
) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO comms_channels (id, channel_type, owner_id) VALUES ($1, 'direct_message', $2)",
        channel_id,
        owner,
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "INSERT INTO comms_channel_agents (channel_id, bot_id, kind, user_id) VALUES ($1, $2, 'direct', $3)",
        channel_id,
        bot_id.as_uuid(),
        owner,
    )
    .execute(pool)
    .await?;
    let principal = bot_id.into_storage_id();
    sqlx::query!(
        "INSERT INTO comms_channel_participants (channel_id, user_id, role) VALUES ($1, $2, 'owner'), ($1, $3, 'member')",
        channel_id,
        owner,
        principal.as_ref(),
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Any member of a persona's team can mint its team-scope token, so the
/// conversation one teammate has with the persona must stay out of it: its
/// channel, and everything shared there, including the conversation's
/// sessions.
#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_scope_does_not_reach_a_direct_agent_conversation(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let shared_channel_id = Uuid::new_v4();
    let conversation_id = Uuid::new_v4();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    insert_pg_bot_channel(&pool, shared_channel_id, "private", None).await?;
    insert_pg_bot_participant(&pool, shared_channel_id, bot_id, "member").await?;
    insert_pg_direct_agent_conversation(&pool, conversation_id, PG_BOT_OWNER, bot_id).await?;
    let repo = PgAccessRepository::new(pool);

    assert_eq!(
        repo.get_team_channel_role(&shared_channel_id, team_id, bot_id)
            .await?,
        ChannelRoleResult::Role(ParticipantRole::Member),
    );
    assert_eq!(
        repo.get_team_channel_role(&conversation_id, team_id, bot_id)
            .await?,
        ChannelRoleResult::NoAccess,
    );
    for (entity_type, stored_type) in [
        (EntityType::Document, "document"),
        (EntityType::AgentSession, "agent_session"),
    ] {
        let in_shared_channel = Uuid::new_v4();
        insert_pg_bot_entity_access(
            &repo.pool,
            in_shared_channel,
            stored_type,
            &shared_channel_id.to_string(),
            "channel",
            AccessLevel::Edit,
        )
        .await?;
        assert_eq!(
            repo.get_team_entity_access(
                bot_id,
                team_id,
                &in_shared_channel.to_string(),
                entity_type,
            )
            .await?,
            Some(AccessLevel::Edit),
            "{stored_type} shared in a channel",
        );

        let in_conversation = Uuid::new_v4();
        insert_pg_bot_entity_access(
            &repo.pool,
            in_conversation,
            stored_type,
            &conversation_id.to_string(),
            "channel",
            AccessLevel::Edit,
        )
        .await?;
        assert_eq!(
            repo.get_team_entity_access(
                bot_id,
                team_id,
                &in_conversation.to_string(),
                entity_type,
            )
            .await?,
            None,
            "{stored_type} shared in the direct conversation",
        );
    }
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_foreign_entity_access_uses_only_team_and_bot_pairs(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::new_v4();
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let bot_principal = bot_id.into_storage_id();
    insert_pg_bot_team(&pool, team_id).await?;
    insert_pg_bot(&pool, bot_id, None, Some(team_id)).await?;
    let repo = PgAccessRepository::new(pool);

    let team_entity_id = Uuid::new_v4();
    insert_foreign_entity(&repo.pool, team_entity_id, &team_id.to_string(), "team").await?;
    let bot_entity_id = Uuid::new_v4();
    insert_foreign_entity(&repo.pool, bot_entity_id, bot_principal.as_ref(), "user").await?;
    let wrong_namespace_id = Uuid::new_v4();
    insert_foreign_entity(
        &repo.pool,
        wrong_namespace_id,
        bot_principal.as_ref(),
        "team",
    )
    .await?;

    for entity_id in [team_entity_id, bot_entity_id] {
        assert!(
            repo.has_team_foreign_entity_access(&entity_id.to_string(), team_id, bot_id)
                .await?
        );
    }
    assert!(
        !repo
            .has_team_foreign_entity_access(&wrong_namespace_id.to_string(), team_id, bot_id)
            .await?
    );
    assert!(
        !repo
            .has_team_foreign_entity_access(&team_entity_id.to_string(), Uuid::new_v4(), bot_id,)
            .await?
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_scoped_repository_methods_reject_malformed_ids(pool: PgPool) -> anyhow::Result<()> {
    let repo = PgAccessRepository::new(pool);
    let bot_id = BotId::new_from_uuid(Uuid::new_v4());
    let team_id = Uuid::new_v4();

    assert!(matches!(
        repo.get_team_entity_access(bot_id, team_id, "bad", EntityType::Document)
            .await,
        Err(AccessError::BadRequest("Invalid entity ID format"))
    ));
    assert!(matches!(
        repo.has_team_foreign_entity_access("bad", team_id, bot_id)
            .await,
        Err(AccessError::BadRequest("Invalid foreign entity ID format"))
    ));
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn team_item_access_rejects_unsupported_entity_type(pool: PgPool) -> anyhow::Result<()> {
    let error = PgAccessRepository::new(pool)
        .get_team_entity_access(
            BotId::new_from_uuid(Uuid::new_v4()),
            Uuid::new_v4(),
            &Uuid::new_v4().to_string(),
            EntityType::CrmCompany,
        )
        .await
        .expect_err("unsupported team item type should be rejected");

    assert!(matches!(
        error,
        AccessError::BadRequest("Unsupported entity type for team item access")
    ));
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn direct_recipients_exclude_inherited_grants(pool: PgPool) -> anyhow::Result<()> {
    let entity_id = Uuid::now_v7();
    for (source, kind) in [
        ("macro|direct@example.com".to_string(), "user"),
        (Uuid::now_v7().to_string(), "team"),
        (Uuid::now_v7().to_string(), "channel"),
    ] {
        sqlx::query!("INSERT INTO entity_access (entity_id, entity_type, source_id, source_type, access_level) VALUES ($1, 'call', $2, $3::text::entity_access_source_type, 'view')", entity_id, source, kind).execute(&pool).await?;
    }
    let repo = PgAccessRepository::new(pool);
    let users = repo
        .get_direct_entity_users(&entity_id, EntityType::Call)
        .await?;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].as_ref(), "macro|direct@example.com");
    Ok(())
}
