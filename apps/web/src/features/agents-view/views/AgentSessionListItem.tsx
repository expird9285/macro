import type { AgentSessionEntity } from '@entity';
import SparkleIcon from '@phosphor/sparkle.svg';
import { Show } from 'solid-js';
import {
  AgentSessionRow,
  AgentSessionStatusIndicator,
} from '../components/AgentSessionRow';
import { conversationState } from '../core/conversation-state';
import { compactAge } from '../core/format-age';
import type { AgentsMode } from '../core/mode';
import { conversationTimestamp } from '../core/recent-conversations';

type Props = {
  entity: AgentSessionEntity;
  surface: 'home' | 'agents';
  active?: boolean;
  unread?: boolean;
  mode?: AgentsMode;
  onOpen?: (event: MouseEvent) => void;
};

/** Persisted list metadata stays fresh through the session update events. */
export function AgentSessionListItem(props: Props) {
  const state = () =>
    conversationState(props.entity.status, props.entity.turnState);

  const hideLeading = () => props.surface === 'agents';
  const workStatusIndicator = () =>
    props.surface === 'agents' ? (
      <AgentSessionStatusIndicator state={state()} unread={props.unread} />
    ) : undefined;

  return (
    <AgentSessionRow
      id={props.entity.id}
      title={props.entity.name || 'Untitled conversation'}
      kind="chat"
      state={state()}
      active={props.active}
      unread={props.unread}
      hideLeading={hideLeading()}
      onOpen={props.onOpen}
      timestamp={compactAge(conversationTimestamp(props.entity))}
      leading={
        <Show
          when={props.surface === 'home'}
          fallback={
            <AgentSessionStatusIndicator
              state={state()}
              unread={props.unread}
            />
          }
        >
          <SparkleIcon />
        </Show>
      }
      trailing={
        <>
          <Show when={props.surface === 'home' && props.unread}>
            <span
              aria-label="Unread"
              class="size-1.5 shrink-0 rounded-full bg-accent"
            />
          </Show>
          {workStatusIndicator()}
        </>
      }
    />
  );
}
