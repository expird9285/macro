use super::*;

#[test]
fn builtin_runtimes_always_bypass_regardless_of_saved_agent_choice() {
    for auto_accept_permissions in [None, Some(false), Some(true)] {
        assert_eq!(
            PermissionPolicyConfig::Persona {
                kind: AgentKind::InMemory,
                harness_allows_bypass: None,
                auto_accept_permissions,
            }
            .resolve(),
            PermissionPolicy::AutoAccept
        );
    }
}

#[test]
fn only_the_in_memory_runtime_is_not_a_coder() {
    assert!(!AgentKind::InMemory.is_coding());
}

#[test]
fn a_turn_with_text_answered_however_it_stopped() {
    for stop in [
        StopReason::EndTurn,
        StopReason::Cancelled,
        StopReason::Failed {
            message: "boom".to_owned(),
            notice: None,
        },
    ] {
        assert_eq!(
            ReplyOutcome::of_turn(&stop, Some("Here you go.".to_owned())),
            ReplyOutcome::Answered("Here you go.".to_owned()),
            "{stop:?}"
        );
    }
}

#[test]
fn a_silent_turn_is_told_by_how_it_stopped() {
    for (stop, expected) in [
        (StopReason::EndTurn, ReplyOutcome::Empty),
        (StopReason::MaxTokens, ReplyOutcome::Empty),
        (StopReason::Refusal, ReplyOutcome::Empty),
        (
            StopReason::Other {
                reason: "mystery".to_owned(),
            },
            ReplyOutcome::Empty,
        ),
        (StopReason::Cancelled, ReplyOutcome::Cancelled),
        (
            StopReason::Failed {
                message: "boom".to_owned(),
                notice: None,
            },
            ReplyOutcome::Failed,
        ),
    ] {
        assert_eq!(ReplyOutcome::of_turn(&stop, None), expected, "{stop:?}");
        assert_eq!(
            ReplyOutcome::of_turn(&stop, Some("  \n".to_owned())),
            expected,
            "blank text is no text: {stop:?}"
        );
    }
}
