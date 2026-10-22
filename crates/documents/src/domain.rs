//! Domain layer: models, ports (trait interfaces), and service implementation.

/// Event-to-activity mappings for this domain.
pub mod activity;
#[cfg(feature = "ai_tools")]
pub mod ai_editing;
pub mod branch_name;
/// A document's comment threads, read through the shared message service.
#[cfg(feature = "ai_tools")]
pub mod comments;
pub mod content;
pub mod docx_export;
/// Unified entity-mutation capability impls.
#[cfg(feature = "service")]
pub mod entity_mutation;
pub mod events;
#[cfg(feature = "ports")]
pub mod markdown_backfill;

#[cfg(feature = "document_create")]
pub mod create;

#[cfg(feature = "document_create")]
pub mod starter;

#[cfg(feature = "ports")]
pub mod upload_finalize;

/// Legacy Office (.doc/.ppt/.xls) upgrades to OpenXML.
#[cfg(feature = "ports")]
pub mod legacy_office_upgrade;

pub mod models;
#[cfg(feature = "axum")]
pub mod permission_token;
pub mod response;

#[cfg(feature = "ports")]
pub mod ports;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "ports")]
pub mod purge;

/// Current Markdown and extracted text, authorized by document view capability.
#[cfg(feature = "ports")]
pub mod text;
