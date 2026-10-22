import { MACRO_AGENT_BOT_ID } from '@core/constant/macroAgent';
import { MACRO_NEW_BOT_ID } from '@core/constant/macroNew';
import type { AgentsMode } from './mode';

/**
 * What an agent is for.
 */
export type AgentKind = 'agent';

export function isCoderHarness(_harness: string | null | undefined): boolean {
  return false;
}

export function kindForHarness(_harness: string | null | undefined): AgentKind {
  return 'agent';
}

/** The first-party bots, whose kind is fixed by what mentioning them does. */
const SYSTEM_BOT_KINDS: ReadonlyMap<string, AgentKind> = new Map([
  [MACRO_AGENT_BOT_ID, 'agent'],
  [MACRO_NEW_BOT_ID, 'agent'],
]);

export function systemBotKind(
  botId: string | null | undefined
): AgentKind | undefined {
  if (!botId) return undefined;
  const bare = botId.startsWith('bot|') ? botId.slice('bot|'.length) : botId;
  return SYSTEM_BOT_KINDS.get(bare);
}

export function modeForKind(_kind?: AgentKind): AgentsMode {
  return 'chat';
}

export function kindForMode(_mode?: AgentsMode): AgentKind {
  return 'agent';
}

/** Display nouns for headings and buttons. */
export function agentKindNoun(_kind?: AgentKind, plural = false): string {
  const noun = 'agent';
  return plural ? `${noun}s` : noun;
}
