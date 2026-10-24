use crate::{ColleagueJoinedMacro, InviteToTeamMetadata};
use invite_email::InviteToMacro;
use notification::domain::models::email_notification_digest::{
    EmailBlockList, NotificationSetBuilder,
};

#[cfg(test)]
mod test;

/// define a blocklist of notification types which will never be templated into a digest email
pub fn digest_email_block_list() -> EmailBlockList {
    EmailBlockList::new::<InviteToTeamMetadata>()
        .append::<InviteToMacro>()
        .append::<invite_email::CallInvite>()
        .append::<ColleagueJoinedMacro>()
}
