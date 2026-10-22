//! Sync-service implementation of the collaborative document storage port.

use model::sync_service::SyncServiceVersionID;
use sync_service_client::SyncServiceClient;

use crate::domain::ports::sync::DocumentSyncPort;

impl DocumentSyncPort for SyncServiceClient {
    async fn exists(&self, document_id: &str) -> anyhow::Result<bool> {
        SyncServiceClient::exists(self, document_id).await
    }

    async fn copy_document(
        &self,
        original_document_id: &str,
        target_document_id: &str,
        version_id: Option<SyncServiceVersionID>,
    ) -> anyhow::Result<()> {
        SyncServiceClient::copy_document(self, original_document_id, target_document_id, version_id)
            .await
    }
}
