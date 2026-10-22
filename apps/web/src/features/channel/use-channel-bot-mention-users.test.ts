import type { Agent } from '@service-storage/generated/schemas/agent';
import type { Bot } from '@service-storage/generated/schemas/bot';
import { createRoot } from 'solid-js';
import { describe, expect, it, vi } from 'vitest';
import {
  availableBotMentionUsers,
  useMessageBotMentionUsers,
} from './use-channel-bot-mention-users';

vi.mock('@queries/channel/channel-bots', () => ({
  useChannelBotsQuery: () => ({ isSuccess: true, data: [] }),
}));
vi.mock('@queries/agents/agents', () => ({
  useAgentsQuery: () => ({
    isSuccess: true,
    data: [
      agent('custom-agent', 'Custom agent', 'all'),
    ],
  }),
}));

const timestamp = '2026-08-27T12:00:00Z';

function bot(id: string, name: string, avatarUrl?: string): Bot {
  return {
    id,
    kind: 'owned',
    name,
    handle: name.toLowerCase().replaceAll(' ', '-'),
    has_agent: true,
    avatar_url: avatarUrl,
    created_at: timestamp,
    updated_at: timestamp,
  };
}

function agent(
  id: string,
  name: string,
  channelScope: Agent['channel_scope'],
  harness = 'in-memory'
): Agent {
  return {
    bot: bot(id, name),
    channel_ids: channelScope === 'all' ? [] : ['channel-1'],
    channel_scope: channelScope,
    default_model: 'model',
    harness,
    instructions: '',
    is_coding: false,
    mcp: { scope: 'owner_connections' },
  };
}

describe('availableBotMentionUsers', () => {
  it('adds all-channel agents without adding selected agents from other channels', () => {
    expect(
      availableBotMentionUsers(
        [bot('installed', 'Installed')],
        [
          agent('global', 'Global', 'all'),
          agent('selected', 'Selected', 'selected'),
        ]
      ).map((user) => user.id)
    ).toEqual(['bot|installed', 'bot|global']);
  });

  it('deduplicates an agent that is also an installed channel bot', () => {
    expect(
      availableBotMentionUsers(
        [bot('global', 'Global')],
        [agent('global', 'Global', 'all')]
      )
    ).toHaveLength(1);
  });

  it('preserves the agent avatar for the mention menu', () => {
    const avatarUrl = 'https://example.com/global-agent.png';

    expect(
      availableBotMentionUsers(
        [],
        [
          {
            ...agent('global', 'Global', 'all'),
            bot: bot('global', 'Global', avatarUrl),
          },
        ]
      )
    ).toEqual([
      {
        id: 'bot|global',
        name: 'Global',
        email: 'Global',
        photoUrl: avatarUrl,
      },
    ]);
  });

  it.each(['document', 'initiative', 'crm_company', 'crm_contact'] as const)(
    'includes channel-selected agents on a %s discussion surface',
    (surface) => {
      const selected = agent('doc-only', 'Doc only', 'selected');
      expect(
        availableBotMentionUsers([], [selected], 'channel').map(
          (u) => u.id
        )
      ).toEqual([]);
      expect(
        availableBotMentionUsers([], [selected], surface).map((u) => u.id)
      ).toEqual(['bot|doc-only']);
    }
  );

  it('offers owned and team agents in calls regardless of channel scope', () => {
    const owned = agent('owned-agent', 'Owned agent', 'selected');
    const team = agent('team-agent', 'Team agent', 'all');
    expect(
      availableBotMentionUsers([], [owned, team], 'call').map(
        (user) => user.id
      )
    ).toEqual(['bot|owned-agent', 'bot|team-agent']);
  });
});
