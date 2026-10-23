use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use models_properties::EntityType;
use sqs_client::search::{SearchQueueMessage, call::CallRecordMessage};
use tokio_util::sync::CancellationToken;

use super::*;
use crate::domain::jobs::JobProgress;
use crate::domain::models::{PropertySourcePage, SourcePage};
use crate::domain::ports::{PropertyBackfillIndexer, SearchEventPublisher};

/// Records publisher activity. `SearchQueueMessage` doesn't impl `Clone`, so
/// instead of holding the messages we just count batches + total messages.
#[derive(Default)]
struct RecordingPublisher {
    batch_sizes: Mutex<Vec<usize>>,
}

impl RecordingPublisher {
    fn batch_count(&self) -> usize {
        self.batch_sizes.lock().unwrap().len()
    }

    fn batch_sizes(&self) -> Vec<usize> {
        self.batch_sizes.lock().unwrap().clone()
    }
}

impl SearchEventPublisher for RecordingPublisher {
    async fn publish(&self, messages: Vec<SearchQueueMessage>) -> Result<(), BackfillError> {
        self.batch_sizes.lock().unwrap().push(messages.len());
        Ok(())
    }
}

fn msg(id: &str) -> SearchQueueMessage {
    SearchQueueMessage::CallRecord(CallRecordMessage {
        call_id: id.to_string(),
        index_override: None,
    })
}

/// Build a 1:1 page where messages.len() == rows_consumed (the typical case
/// for non-batching sources like calls/chats/channels/documents).
fn page(messages: Vec<SearchQueueMessage>) -> SourcePage {
    let rows_consumed = messages.len();
    SourcePage {
        messages,
        rows_consumed,
    }
}

fn detached() -> (JobProgress, CancellationToken) {
    (JobProgress::detached(), CancellationToken::new())
}

/// Fake source for the cursor-based loop. Pages out a programmed
/// sequence and records the cursor it was called with each time so
/// tests can assert the cursor advances correctly.
struct CursorFakeSource {
    pages: Mutex<std::collections::VecDeque<(SourcePage, Option<usize>)>>,
    cursors: Mutex<Vec<Option<usize>>>,
}

impl CursorFakeSource {
    fn new(pages: Vec<(SourcePage, Option<usize>)>) -> Self {
        Self {
            pages: Mutex::new(pages.into_iter().collect()),
            cursors: Mutex::new(Vec::new()),
        }
    }

    async fn fetch_page(
        &self,
        cursor: Option<usize>,
    ) -> Result<(SourcePage, Option<usize>), BackfillError> {
        self.cursors.lock().unwrap().push(cursor);
        Ok(self
            .pages
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| (SourcePage::empty(), None)))
    }

    fn observed_cursors(&self) -> Vec<Option<usize>> {
        self.cursors.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn cursor_drain_threads_cursor_through_pages() {
    // Three pages: first call passes None; subsequent calls pass the
    // cursor each previous page returned. Loop stops on empty page.
    let source = CursorFakeSource::new(vec![
        (
            page((0..5).map(|i| msg(&format!("p1-{i}"))).collect()),
            Some(10),
        ),
        (
            page((0..5).map(|i| msg(&format!("p2-{i}"))).collect()),
            Some(20),
        ),
        (
            page((0..3).map(|i| msg(&format!("p3-{i}"))).collect()),
            Some(30),
        ),
    ]);
    let publisher = RecordingPublisher::default();
    let (progress, cancel) = detached();

    let receipt = drain_source_with_cursor(&publisher, &progress, &cancel, |cursor| {
        source.fetch_page(cursor)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 13);
    assert_eq!(publisher.batch_sizes(), vec![5, 5, 3]);
    // Cursors: None (first), Some(10), Some(20), Some(30) (empty terminator).
    assert_eq!(
        source.observed_cursors(),
        vec![None, Some(10), Some(20), Some(30)]
    );
}

#[tokio::test]
async fn cursor_drain_stops_when_non_empty_page_returns_no_cursor() {
    // Defensive path: source returns a non-empty page but `None` for the
    // next cursor (e.g. the last row's sort-key column came back NULL).
    // The loop must treat this as terminator — passing `None` back into
    // `fetch` would restart pagination from page one.
    let source = CursorFakeSource::new(vec![
        (
            page((0..3).map(|i| msg(&format!("p1-{i}"))).collect()),
            None,
        ),
        (
            page((0..3).map(|i| msg(&format!("never-{i}"))).collect()),
            Some(99),
        ),
    ]);
    let publisher = RecordingPublisher::default();
    let (progress, cancel) = detached();

    let receipt = drain_source_with_cursor(&publisher, &progress, &cancel, |cursor| {
        source.fetch_page(cursor)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 3);
    assert_eq!(publisher.batch_sizes(), vec![3]);
    // Only the first fetch happens; the loop bails before re-entering with `None`.
    assert_eq!(source.observed_cursors(), vec![None]);
}

#[tokio::test]
async fn cursor_drain_publish_failure_counts_only_completed_pages() {
    struct FailSecondPublish(AtomicUsize);
    impl SearchEventPublisher for FailSecondPublish {
        async fn publish(&self, _: Vec<SearchQueueMessage>) -> Result<(), BackfillError> {
            if self.0.fetch_add(1, Ordering::Relaxed) == 1 {
                return Err(BackfillError::Publish(anyhow::anyhow!("queue unavailable")));
            }
            Ok(())
        }
    }
    let source = CursorFakeSource::new(vec![
        (page(vec![msg("first")]), Some(1)),
        (page(vec![msg("failed")]), Some(2)),
        (page(vec![msg("not-fetched")]), Some(3)),
    ]);
    let publisher = FailSecondPublish(AtomicUsize::new(0));
    let (progress, cancel) = detached();
    let error = drain_source_with_cursor(&publisher, &progress, &cancel, |cursor| {
        source.fetch_page(cursor)
    })
    .await
    .unwrap_err();
    assert!(matches!(error, BackfillError::Publish(_)));
    assert_eq!(progress.local_count(), 1);
    assert_eq!(source.observed_cursors(), vec![None, Some(1)]);
}

#[tokio::test]
async fn cursor_drain_stops_on_empty_page() {
    let source = CursorFakeSource::new(vec![]);
    let publisher = RecordingPublisher::default();
    let (progress, cancel) = detached();

    let receipt = drain_source_with_cursor(&publisher, &progress, &cancel, |cursor| {
        source.fetch_page(cursor)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 0);
    assert_eq!(publisher.batch_count(), 0);
    // Probed once with no cursor, got empty, terminated.
    assert_eq!(source.observed_cursors(), vec![None]);
}

#[tokio::test]
async fn cursor_drain_cancellation_stops_between_pages() {
    let source = CursorFakeSource::new(vec![
        (
            page((0..3).map(|i| msg(&format!("a{i}"))).collect()),
            Some(7),
        ),
        (
            page((0..3).map(|i| msg(&format!("never-{i}"))).collect()),
            Some(99),
        ),
    ]);
    let (progress, cancel) = detached();
    let cancel_clone = cancel.clone();
    struct CancellingPublisher {
        cancel: CancellationToken,
        seen: Mutex<Vec<usize>>,
    }
    impl SearchEventPublisher for CancellingPublisher {
        async fn publish(&self, messages: Vec<SearchQueueMessage>) -> Result<(), BackfillError> {
            self.seen.lock().unwrap().push(messages.len());
            self.cancel.cancel();
            Ok(())
        }
    }
    let publisher = CancellingPublisher {
        cancel: cancel_clone,
        seen: Mutex::new(Vec::new()),
    };

    let receipt = drain_source_with_cursor(&publisher, &progress, &cancel, |cursor| {
        source.fetch_page(cursor)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 3);
    // First page fetched + published; cancellation stops the second fetch.
    assert_eq!(source.observed_cursors(), vec![None]);
}

struct FakePropertySource {
    pages: Mutex<VecDeque<PropertySourcePage>>,
    offsets: Mutex<Vec<usize>>,
    entity_type: EntityType,
}

impl FakePropertySource {
    fn new(entity_type: EntityType, pages: Vec<PropertySourcePage>) -> Self {
        Self {
            pages: Mutex::new(pages.into_iter().collect()),
            offsets: Mutex::new(Vec::new()),
            entity_type,
        }
    }

    async fn fetch_page(&self, offset: usize) -> Result<PropertySourcePage, BackfillError> {
        self.offsets.lock().unwrap().push(offset);
        Ok(self
            .pages
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| PropertySourcePage::empty(self.entity_type)))
    }

    fn observed_offsets(&self) -> Vec<usize> {
        self.offsets.lock().unwrap().clone()
    }
}

#[derive(Default)]
struct RecordingPropertyIndexer {
    entities: Mutex<Vec<(String, EntityType)>>,
}

impl RecordingPropertyIndexer {
    fn indexed_count(&self) -> usize {
        self.entities.lock().unwrap().len()
    }

    fn indexed_entities(&self) -> Vec<(String, EntityType)> {
        self.entities.lock().unwrap().clone()
    }
}

impl PropertyBackfillIndexer for RecordingPropertyIndexer {
    async fn reindex(&self, entity_id: &str, entity_type: EntityType) -> Result<(), BackfillError> {
        self.entities
            .lock()
            .unwrap()
            .push((entity_id.to_owned(), entity_type));
        Ok(())
    }
}

fn property_page(
    entity_type: EntityType,
    entity_ids: impl IntoIterator<Item = impl Into<String>>,
    rows_consumed: usize,
) -> PropertySourcePage {
    PropertySourcePage {
        entity_ids: entity_ids.into_iter().map(Into::into).collect(),
        entity_type,
        rows_consumed,
    }
}

#[tokio::test]
async fn property_backfill_drains_multiple_pages() {
    let entity_type = EntityType::Document;
    let source = FakePropertySource::new(
        entity_type,
        vec![
            property_page(entity_type, ["one", "two"], 2),
            property_page(entity_type, ["three"], 1),
        ],
    );
    let indexer = RecordingPropertyIndexer::default();
    let (progress, cancel) = detached();

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 3);
    assert_eq!(
        indexer.indexed_entities(),
        vec![
            ("one".to_owned(), EntityType::Document),
            ("two".to_owned(), EntityType::Document),
            ("three".to_owned(), EntityType::Document),
        ]
    );
    assert_eq!(source.observed_offsets(), vec![0, 2, 3]);
}

#[tokio::test]
async fn property_backfill_advances_offset_by_rows_consumed() {
    let entity_type = EntityType::Thread;
    let source = FakePropertySource::new(
        entity_type,
        vec![
            property_page(entity_type, ["one", "two"], 100),
            property_page(entity_type, ["three"], 40),
        ],
    );
    let indexer = RecordingPropertyIndexer::default();
    let (progress, cancel) = detached();

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 140);
    assert_eq!(indexer.indexed_count(), 3);
    assert_eq!(source.observed_offsets(), vec![0, 100, 140]);
}

#[tokio::test]
async fn property_backfill_updates_progress_after_successful_page() {
    let entity_type = EntityType::Project;
    let source = FakePropertySource::new(
        entity_type,
        vec![property_page(entity_type, ["one", "two"], 7)],
    );
    let indexer = RecordingPropertyIndexer::default();
    let (progress, cancel) = detached();

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 7);
    assert_eq!(progress.local_count(), 7);
}

#[tokio::test]
async fn property_backfill_empty_page_indexes_nothing() {
    let entity_type = EntityType::Task;
    let source = FakePropertySource::new(entity_type, Vec::new());
    let indexer = RecordingPropertyIndexer::default();
    let (progress, cancel) = detached();

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 0);
    assert_eq!(progress.local_count(), 0);
    assert_eq!(indexer.indexed_count(), 0);
    assert_eq!(source.observed_offsets(), vec![0]);
}

struct CancellingPropertyIndexer {
    cancel: CancellationToken,
    indexed: AtomicUsize,
}

impl PropertyBackfillIndexer for CancellingPropertyIndexer {
    async fn reindex(
        &self,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<(), BackfillError> {
        self.indexed.fetch_add(1, Ordering::Relaxed);
        self.cancel.cancel();
        Ok(())
    }
}

#[tokio::test]
async fn property_backfill_cancellation_stops_between_pages() {
    let entity_type = EntityType::Chat;
    let source = FakePropertySource::new(
        entity_type,
        vec![
            property_page(entity_type, ["indexed"], 1),
            property_page(entity_type, ["not-indexed"], 1),
        ],
    );
    let (progress, cancel) = detached();
    let indexer = CancellingPropertyIndexer {
        cancel: cancel.clone(),
        indexed: AtomicUsize::new(0),
    };

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 1);
    assert_eq!(progress.local_count(), 1);
    assert_eq!(indexer.indexed.load(Ordering::Relaxed), 1);
    assert_eq!(source.observed_offsets(), vec![0]);
}

struct FailingPropertyIndexer;

impl PropertyBackfillIndexer for FailingPropertyIndexer {
    async fn reindex(
        &self,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<(), BackfillError> {
        Err(BackfillError::Reindex(anyhow::anyhow!("reindex failed")))
    }
}

#[tokio::test]
async fn property_backfill_reindex_failure_propagates_without_progress() {
    let entity_type = EntityType::Channel;
    let source = FakePropertySource::new(
        entity_type,
        vec![property_page(entity_type, ["failure"], 1)],
    );
    let indexer = FailingPropertyIndexer;
    let (progress, cancel) = detached();

    let error = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap_err();

    assert!(matches!(error, BackfillError::Reindex(_)));
    assert_eq!(progress.local_count(), 0);
    assert_eq!(source.observed_offsets(), vec![0]);
}

#[derive(Default)]
struct ConcurrencyTrackingPropertyIndexer {
    active: AtomicUsize,
    maximum_active: AtomicUsize,
    completed: AtomicUsize,
}

impl PropertyBackfillIndexer for ConcurrencyTrackingPropertyIndexer {
    async fn reindex(
        &self,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<(), BackfillError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum_active.fetch_max(active, Ordering::SeqCst);

        tokio::time::sleep(Duration::from_millis(5)).await;

        self.active.fetch_sub(1, Ordering::SeqCst);
        self.completed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn property_backfill_limits_reindexes_to_twenty_simultaneous_operations() {
    let entity_type = EntityType::Document;
    let entity_ids = (0..45).map(|index| format!("entity-{index}"));
    let source = FakePropertySource::new(
        entity_type,
        vec![property_page(entity_type, entity_ids, 45)],
    );
    let indexer = ConcurrencyTrackingPropertyIndexer::default();
    let (progress, cancel) = detached();

    let receipt = drain_property_source(&indexer, &progress, &cancel, |offset| {
        source.fetch_page(offset)
    })
    .await
    .unwrap();

    assert_eq!(receipt.enqueued, 45);
    assert_eq!(indexer.completed.load(Ordering::SeqCst), 45);
    assert_eq!(indexer.maximum_active.load(Ordering::SeqCst), 20);
}
