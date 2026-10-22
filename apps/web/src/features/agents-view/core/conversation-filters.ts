import { match } from 'ts-pattern';
import {
  type ConversationState,
  conversationState,
} from './conversation-state';
import type { AgentsMode } from './mode';
import type { AgentConversationEntity } from './recent-conversations';

export type StatusFilter = 'waiting' | 'working' | 'unread' | 'idle';
export type ConversationGrouping = 'none' | 'status';

/** Values are ORed inside a category and categories are ANDed. */
export type ConversationFilters = {
  status: StatusFilter[];
};

export type ConversationFilterCategory = keyof ConversationFilters;

export const EMPTY_CONVERSATION_FILTERS: ConversationFilters = {
  status: [],
};

/** What the sidebar knows about a row, independent of its entity shape. */
export type ConversationFacts = {
  mode: AgentsMode;
  state: ConversationState;
  unread: boolean;
};

export function conversationFacts(
  conversation: AgentConversationEntity,
  mode: AgentsMode,
  unread: boolean
): ConversationFacts {
  if (conversation.type !== 'agent_session')
    return { mode, state: 'dormant', unread };
  return {
    mode,
    state: conversationState(conversation.status, conversation.turnState),
    unread,
  };
}

function hasStatus(facts: ConversationFacts, status: StatusFilter): boolean {
  return match(status)
    .with('waiting', () => facts.state === 'waiting')
    .with(
      'working',
      () => facts.state === 'working' || facts.state === 'starting'
    )
    .with('idle', () => facts.state === 'dormant')
    .with('unread', () => facts.unread)
    .exhaustive();
}

function matchesCategory<Value>(
  selected: readonly Value[],
  test: (value: Value) => boolean
): boolean {
  return selected.length === 0 || selected.some(test);
}

export function conversationMatchesFilters(
  facts: ConversationFacts,
  filters: ConversationFilters
): boolean {
  return matchesCategory(filters.status, (status) => hasStatus(facts, status));
}

export function activeFilterCount(filters: ConversationFilters): number {
  return filters.status.length;
}

export type ConversationGroup<Item> = {
  id: string;
  /** Undefined when the list is not grouped and needs no heading. */
  label: string | undefined;
  items: Item[];
};

type GroupDefinition = {
  id: string;
  label: string;
  contains: (facts: ConversationFacts) => boolean;
};

const STATUS_GROUPS: GroupDefinition[] = [
  {
    id: 'waiting',
    label: 'Needs you',
    contains: (facts) => facts.state === 'waiting',
  },
  {
    id: 'working',
    label: 'Working',
    contains: (facts) =>
      facts.state === 'working' || facts.state === 'starting',
  },
  {
    id: 'recent',
    label: 'Recent',
    contains: (facts) => facts.state === 'dormant',
  },
];

/** Splits an ordered list into headed sections, dropping empty ones. */
export function groupConversations<Item>(
  items: readonly Item[],
  factsOf: (item: Item) => ConversationFacts,
  grouping: ConversationGrouping
): ConversationGroup<Item>[] {
  if (grouping === 'none')
    return [{ id: 'all', label: undefined, items: [...items] }];
  const definitions = STATUS_GROUPS;
  return definitions
    .map((definition) => ({
      id: definition.id,
      label: definition.label,
      items: items.filter((item) => definition.contains(factsOf(item))),
    }))
    .filter((group) => group.items.length > 0);
}
