use super::*;

#[test]
fn legacy_not_uploaded_is_pending() {
    assert_eq!(
        DocumentContent::from_legacy_uploaded(false, Some(FileType::Pdf)),
        DocumentContent {
            state: DocumentContentState::Pending,
            location: None,
        }
    );
}

#[test]
fn legacy_uploaded_markdown_location_is_unknown() {
    assert_eq!(
        DocumentContent::from_legacy_uploaded(true, Some(FileType::Md)),
        DocumentContent::ready(DocumentContentLocation::Unknown)
    );
}

#[test]
fn legacy_uploaded_non_markdown_uses_object_storage() {
    assert_eq!(
        DocumentContent::from_legacy_uploaded(true, Some(FileType::Pdf)),
        DocumentContent::ready(DocumentContentLocation::ObjectStorage)
    );
}
