import { describe, expect, it } from 'vitest';
import {
  type AgentsRoute,
  agentsRouteFromSegments,
  agentsRouteId,
  agentsRouteSegments,
  parseAgentsRoute,
} from './route';

describe('Agents workspace routes', () => {
  it.each<[AgentsRoute, string]>([
    [
      {
        mode: 'chat',
        conversation: { type: 'agent_session', id: 'session-1' },
      },
      'agents',
    ],
    [
      { mode: 'chat', conversation: { type: 'chat', id: 'chat-1' } },
      'agent-chats',
    ],
  ])('round trips the selected conversation: %j', (route, section) => {
    const id = agentsRouteId(route);
    expect(parseAgentsRoute(id)).toEqual(route);
    expect(agentsRouteSegments(id)).toEqual([section, route.conversation.id]);
    expect(agentsRouteFromSegments(section, route.conversation.id)).toBe(id);
  });

  it('normalizes legacy coders routes and component IDs to agents (FE-51)', () => {
    const legacyCoderId = 'agents-session~coders~session-legacy';
    expect(parseAgentsRoute(legacyCoderId)).toEqual({
      mode: 'chat',
      conversation: { type: 'agent_session', id: 'session-legacy' },
    });
    expect(agentsRouteSegments(legacyCoderId)).toEqual(['agents', 'session-legacy']);
    expect(agentsRouteFromSegments('coders', 'session-legacy')).toBe('agents-session~agents~session-legacy');
  });

  it('does not claim unrelated routes or malformed component identities', () => {
    expect(agentsRouteFromSegments('md', 'document-1')).toBeUndefined();
    expect(parseAgentsRoute('agents')).toBeUndefined();
    expect(parseAgentsRoute('agents-session~agents~')).toBeUndefined();
    expect(parseAgentsRoute('agents-session~agents~id~extra')).toBeUndefined();
  });
});
