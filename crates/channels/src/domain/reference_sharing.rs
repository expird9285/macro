//! Reference sharing policy, independent of persistence and transport.

use super::models::ReferencedShareItemType;
use entity_access::domain::models::AccessLevel;
use model_file_type::FileType;

/// Only owners may share session references, and automatic sharing grants view
/// access. Sending a reference never grants control of an agent session, and a
/// direct agent conversation's sessions, private to their owner, are never
/// shared this way: the share adapter skips them before asking.
///
/// A PDF grants comment access so channel members can annotate it, capped at
/// the sharer's own access. `file_type` is the referenced document's file type.
///
/// Posting a form lets the channel respond to it: anyone who can see the form
/// grants View, whatever their own level. Edit is never granted by posting.
#[cfg_attr(
    not(feature = "outbound"),
    allow(dead_code, reason = "only the outbound share adapter grants access")
)]
pub(crate) fn grant_level(
    item_type: ReferencedShareItemType,
    file_type: Option<FileType>,
    access: Option<AccessLevel>,
) -> Option<AccessLevel> {
    match (item_type, access) {
        (ReferencedShareItemType::AgentSession, Some(AccessLevel::Owner)) => {
            Some(AccessLevel::View)
        }
        (ReferencedShareItemType::Form, Some(_)) => Some(AccessLevel::View),
        (ReferencedShareItemType::AgentSession, _) | (_, None) => None,
        (ReferencedShareItemType::Document, Some(level)) if file_type == Some(FileType::Pdf) => {
            Some(level.min(AccessLevel::Comment))
        }
        (_, Some(_)) => Some(AccessLevel::View),
    }
}

#[cfg(test)]
mod test;
