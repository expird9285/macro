import { MACRO_AGENT_BOT_ID } from '@core/constant/macroAgent';
import { describe, expect, it } from 'vitest';
import {
  buildAgentRoster,
  kindForBot,
  MACRO_PERSONA_ID,
  type PersistedAgentLike,
  type RosterInput,
  rosterForAgentPicker,
} from './roster';

function persisted(
  overrides: Partial<PersistedAgentLike> & { harness: string }
): PersistedAgentLike {
  return {
    bot: {
      id: `bot-${overrides.harness}`,
      name: 'Agent',
      handle: 'agent',
      owner: { type: 'user', user_id: 'macro|me@example.com' },
    },
    default_model: 'claude-sonnet-5',
    ...overrides,
  };
}

const EMPTY: RosterInput = {
  agents: [],
};

describe('buildAgentRoster', () => {
  it('leads with Macro chat agent', () => {
    const roster = buildAgentRoster(EMPTY);
    expect(roster.map((agent) => [agent.id, agent.kind])).toEqual([
      [MACRO_PERSONA_ID, 'agent'],
    ]);
    expect(roster[0]?.botId).toBeUndefined();
    expect(roster.every((agent) => agent.share === 'system')).toBe(true);
  });

  it('classifies saved agents and marks them connected', () => {
    const roster = buildAgentRoster({
      ...EMPTY,
      agents: [
        persisted({ harness: 'in-memory' }),
        persisted({
          harness: 'macro-inmem',
          bot: {
            id: 'bot-team',
            name: 'Reviewer',
            handle: 'reviewer',
            owner: { type: 'team', team_id: 't1' },
          },
        }),
      ],
    });
    const saved = roster.slice(1);
    expect(saved.map((agent) => agent.kind)).toEqual(['agent', 'agent']);
    expect(saved[0]?.runtime).toEqual({
      label: 'Macro Agent',
      connected: true,
    });
    expect(saved[0]?.unavailableReason).toBeUndefined();
    expect(saved[1]?.share).toBe('team');
    expect(saved[1]?.runtime).toEqual({ label: 'Macro Agent', connected: true });
  });
});

describe('rosterForAgentPicker and kindForBot', () => {
  const roster = buildAgentRoster({
    ...EMPTY,
    agents: [
      persisted({ harness: 'in-memory' }),
      persisted({
        harness: 'in-memory',
        bot: { id: MACRO_AGENT_BOT_ID, name: 'Macro', handle: 'macro' },
      }),
      persisted({
        harness: 'in-memory',
        bot: {
          id: `bot|${MACRO_AGENT_BOT_ID}`,
          name: 'Macro',
          handle: 'macro',
        },
      }),
      persisted({
        harness: 'in-memory',
        bot: { id: 'saved-macro-name', name: 'Macro', handle: 'my-macro' },
      }),
    ],
  });

  it('excludes built-in Macro identity from picker', () => {
    expect(rosterForAgentPicker(roster).map((a) => a.id)).toEqual([
      'bot-in-memory',
      'saved-macro-name',
    ]);
  });

  it('resolves a session bot to its kind', () => {
    expect(kindForBot('bot-in-memory', roster)).toBe('agent');
    expect(kindForBot('someone-elses-bot', roster)).toBe('agent');
    expect(kindForBot(undefined, roster)).toBe('agent');
  });
});
