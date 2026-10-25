//! Handler for `POST /documents`.

use std::str::FromStr;

use axum::{Json, extract::State};
use entity_access::domain::ports::EntityAccessService;
use entity_access::inbound::axum_extractors::ProjectBodyAccessLevelExtractorV2;
use entity_registry::CreationPrincipalExtractor;
use macro_authorization::MacroAuthorizationService;
use model::document::response::CreateDocumentRequest;
use model::document::{FileType, FileTypeExt};
use model_owner::CreationPrincipal;
use models_permissions::share_permission::access_level::EditAccessLevel;

use super::DocumentRouterState;
use crate::domain::models::{DocumentError, InitialLinkShare, NewDocument};
use crate::domain::ports::DocumentService;
use crate::domain::response::CreateDocumentResponse;

/// Handler for `POST /documents`.
///
/// Creates a new document, generates an S3 presigned upload URL, and returns
/// the document metadata with the URL for the client to upload to.
#[utoipa::path(
    tag = "document",
    post,
    path = "/documents",
    operation_id = "create_document",
    request_body = CreateDocumentRequest,
    responses(
        (status = 200, body = inline(CreateDocumentResponse)),
        (status = 400, body = model_error_response::ErrorResponse),
        (status = 401, body = model_error_response::ErrorResponse),
        (status = 409, body = model_error_response::ErrorResponse),
        (status = 422, description = "Document name exceeds the maximum length", body = model_error_response::ErrorResponse),
        (status = 500, body = model_error_response::ErrorResponse),
    )
)]
#[tracing::instrument(skip(state, project))]
pub async fn create_document_handler<
    T: DocumentService,
    Svc: EntityAccessService,
    Auth: MacroAuthorizationService,
>(
    State(state): State<DocumentRouterState<T, Svc, Auth>>,
    CreationPrincipalExtractor { principal, .. }: CreationPrincipalExtractor<Auth>,
    project: ProjectBodyAccessLevelExtractorV2<EditAccessLevel, CreateDocumentRequest, Svc, Auth>,
) -> Result<Json<CreateDocumentResponse>, DocumentError> {
    let req = project.into_inner();

    let job_id = req.job_id.clone();
    let response_data = state
        .service
        .create_document(&principal, new_document(req), job_id)
        .await?;

    Ok(Json(CreateDocumentResponse {
        error: false,
        data: response_data,
    }))
}

fn new_document(req: CreateDocumentRequest) -> NewDocument {
    let user_provided_file_type: Option<FileType> = req
        .file_type
        .as_deref()
        .and_then(|f| FileType::from_str(f).ok());

    let (document_name, file_type) = match user_provided_file_type {
        Some(file_type) => {
            let document_name = FileType::clean_document_name(&req.document_name);
            (document_name.unwrap_or(req.document_name), Some(file_type))
        }
        None => match FileType::split_suffix_match(req.document_name.as_str()) {
            Some((file_name, extension)) => {
                let file_type: Option<FileType> = FileType::from_str(extension).ok();
                (file_name.to_string(), file_type)
            }
            None => (req.document_name, None),
        },
    };

    // Log if the user-provided mime type does not match the file type
    if let (Some(ft), Some(user_mime_type)) = (file_type, &req.mime_type)
        && *user_mime_type != ft.mime_type()
    {
        tracing::warn!(
            file_type=?ft,
            mime_type=?user_mime_type,
            "provided mime type does not match file type"
        );
    }

    NewDocument {
        id: req.id,
        sha: req.sha,
        document_name,
        file_type,
        project_id: req.project_id,
        team_id: req.team_id,
        share_with_team: false,
        created_at: req.created_at,
        sub_type: req
            .is_task
            .then_some(document_sub_type::DocumentSubType::Task),
        skip_history: req.skip_history,
        initial_link_share: InitialLinkShare::EntityDefault,
    }
}
