use super::*;
use crate::domain::jobs::JobProgress;
use crate::domain::ports::{PropertyBackfillIndexer, SearchEventPublisher};
use crate::domain::service::{BackfillOrchestrator, BackfillService};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

fn source(db: PgPool, primary_db: PgPool) -> PgBackfillSource {
    PgBackfillSource::new(
        db,
        primary_db,
        BackfillPageSizes {
            calls: 1,
            chats: 1,
            channels: 1,
            documents: 1,
            projects: 1,
        },
    )
}

async fn closed_pool() -> PgPool {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@localhost/unused")
        .unwrap();
    pool.close().await;
    pool
}

#[derive(Clone, Default)]
struct ChannelPublisher(Arc<Mutex<Vec<(String, String)>>>);

impl SearchEventPublisher for ChannelPublisher {
    async fn publish(&self, messages: Vec<SearchQueueMessage>) -> Result<(), BackfillError> {
        for message in messages {
            let SearchQueueMessage::ChannelMessageUpdate(update) = message else {
                panic!("expected channel update");
            };
            assert_eq!(update.index_override.as_deref(), Some("channels_repair"));
            self.0
                .lock()
                .unwrap()
                .push((update.channel_id, update.message_id));
        }
        Ok(())
    }
}

struct UnusedPropertyIndexer;

impl PropertyBackfillIndexer for UnusedPropertyIndexer {
    async fn reindex(&self, _: &str, _: EntityType) -> Result<(), BackfillError> {
        panic!("channel publication must not call a direct indexer");
    }
}

#[tokio::test]
async fn empty_channel_scope_never_reads_or_publishes() {
    let db = closed_pool().await;
    let source = source(db.clone(), db);
    let request = ChannelBackfillRequest {
        channel_ids: Some(vec![]),
        ..Default::default()
    };
    let (page, cursor) = source.fetch_channels(&request, None).await.unwrap();
    assert!(page.messages.is_empty());
    assert_eq!(page.rows_consumed, 0);
    assert!(cursor.is_none());

    let publisher = ChannelPublisher::default();
    let service = BackfillOrchestrator::new(source, publisher.clone(), UnusedPropertyIndexer);
    let progress = Arc::new(JobProgress::detached());
    let receipt = service
        .backfill_channels(request, progress.clone(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(receipt.enqueued, 0);
    assert_eq!(progress.local_count(), 0);
    assert!(publisher.0.lock().unwrap().is_empty());
}

#[sqlx::test(
    migrations = "../../crates/macro_db_client/migrations",
    fixtures(
        path = "../../../../../crates/comms_db_client/fixtures",
        scripts("channels")
    )
)]
async fn scoped_backfill_publishes_only_primary_channel_history_and_can_replay(pool: PgPool) {
    use comms_db_client::messages::create_message::{CreateMessageOptions, create_message};
    let channel = uuid::Uuid::from_u128(0x11111111_1111_1111_1111_111111111111);
    let other = uuid::Uuid::from_u128(0x33333333_3333_3333_3333_333333333333);
    let mut expected = Vec::new();
    for channel_id in [channel, other, channel] {
        let message = create_message(
            &pool,
            CreateMessageOptions {
                channel_id,
                sender_id: "macro|user1@test.com".into(),
                content: "history".into(),
                thread_id: None,
            },
        )
        .await
        .unwrap();
        if channel_id == channel {
            expected.push((channel.to_string(), message.id.to_string()));
        }
    }
    let replica = closed_pool().await;
    let backfill_source = source(replica.clone(), pool.clone());
    let publisher = ChannelPublisher::default();
    let service =
        BackfillOrchestrator::new(backfill_source, publisher.clone(), UnusedPropertyIndexer);
    let request = ChannelBackfillRequest {
        channel_ids: Some(vec![channel]),
        index_override: Some("channels_repair".into()),
        ..Default::default()
    };
    // A lost receipt or stale job can restart the same scope; publication is
    // repeatable and downstream upserts deduplicate by message ID.
    for _ in 0..2 {
        let progress = Arc::new(JobProgress::detached());
        let receipt = service
            .backfill_channels(request.clone(), progress.clone(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(receipt.enqueued, 2);
        assert_eq!(progress.local_count(), 2);
        assert_eq!(std::mem::take(&mut *publisher.0.lock().unwrap()), expected);
    }

    // Conversely, global scans retain the configured backfill pool. A closed
    // primary cannot interfere with an unscoped request.
    let global = source(pool, replica);
    let (page, _) = global
        .fetch_channels(&ChannelBackfillRequest::default(), None)
        .await
        .unwrap();
    assert_eq!(page.rows_consumed, 1);
    assert!(matches!(
        global.fetch_channels(&request, None).await,
        Err(BackfillError::Source(_))
    ));
}
