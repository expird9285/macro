import { MACRO_AGENT_BOT_ID } from '@core/constant/macroAgent';
import { describe, expect, it } from 'vitest';
import {
  agentKindNoun,
  isCoderHarness,
  kindForHarness,
  kindForMode,
  modeForKind,
  systemBotKind,
} from './agent-kind';

describe('isCoderHarness', () => {
  it('treats all harnesses as non-coder', () => {
    expect(isCoderHarness('in-memory')).toBe(false);
    expect(isCoderHarness('macro-inmem')).toBe(false);
    expect(isCoderHarness(undefined)).toBe(false);
    expect(isCoderHarness('')).toBe(false);
    expect(isCoderHarness('cursor')).toBe(false);
    expect(isCoderHarness('macrod')).toBe(false);
    expect(kindForHarness('in-memory')).toBe('agent');
  });
});

describe('systemBotKind', () => {
  it('knows the first-party bots in either id form', () => {
    expect(systemBotKind(MACRO_AGENT_BOT_ID)).toBe('agent');
    expect(systemBotKind(`bot|${MACRO_AGENT_BOT_ID}`)).toBe('agent');
  });

  it('has no opinion about other bots', () => {
    expect(
      systemBotKind('11111111-1111-1111-1111-111111111111')
    ).toBeUndefined();
    expect(systemBotKind(undefined)).toBeUndefined();
  });
});

describe('modes and kinds', () => {
  it('maps to chat and agent', () => {
    expect(modeForKind(kindForMode('chat'))).toBe('chat');
  });

  it('names the kind', () => {
    expect(agentKindNoun('agent')).toBe('agent');
    expect(agentKindNoun('agent', true)).toBe('agents');
  });
});
