import type { AgentsMode } from './mode';
import type { AgentConversationTarget } from './recent-conversations';

const PREFIX = 'agents-session~';

export type AgentsRoute = {
  mode?: AgentsMode;
  conversation: AgentConversationTarget;
};

/** Distinct component identities preserve each conversation in split history. */
export function agentsRouteId(route: AgentsRoute): string {
  const section =
    route.conversation.type === 'chat'
      ? 'agent-chats'
      : 'agents';
  return `${PREFIX}${section}~${route.conversation.id}`;
}

export function parseAgentsRoute(id: string): AgentsRoute | undefined {
  if (!id.startsWith(PREFIX)) return;
  const [section, conversationId, extra] = id.slice(PREFIX.length).split('~');
  if (!conversationId || extra !== undefined) return;
  if (section !== 'agents' && section !== 'coders' && section !== 'agent-chats')
    return;
  return {
    mode: 'chat',
    conversation: {
      type: section === 'agent-chats' ? 'chat' : 'agent_session',
      id: conversationId,
    },
  };
}

export function agentsRouteSegments(id: string): string[] | undefined {
  const parsed = parseAgentsRoute(id);
  if (!parsed) return;
  const section = parsed.conversation.type === 'chat' ? 'agent-chats' : 'agents';
  return [section, parsed.conversation.id];
}

export function agentsRouteFromSegments(
  section: string,
  id: string
): string | undefined {
  const normalizedSection = section === 'coders' ? 'agents' : section;
  const componentId = `${PREFIX}${normalizedSection}~${id}`;
  return parseAgentsRoute(componentId) ? componentId : undefined;
}
