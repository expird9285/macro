import { describe, expect, it } from 'vitest';
import {
  harnessDisplayName,
  harnessTitle,
  sessionHarnessTitle,
  sessionRepositoryUrl,
  showsSessionHarness,
} from './compose-agent-session-options';

describe('sessionRepositoryUrl', () => {
  it('is always undefined now that coding sessions are removed', () => {
    expect(sessionRepositoryUrl(undefined)).toBe(undefined);
    expect(sessionRepositoryUrl({ harness: 'in-memory' })).toBe(undefined);
  });
});

describe('harnessDisplayName', () => {
  it('names Macro runtimes after the product', () => {
    expect(harnessDisplayName('in-memory')).toBe('Macro Agent');
    expect(harnessDisplayName('macro-inmem')).toBe('Macro Agent');
    expect(harnessDisplayName('sandbox')).toBe('Macro Agent');
  });

  it('leaves other harness names alone', () => {
    expect(harnessDisplayName('my-laptop')).toBe('my-laptop');
  });
});

describe('harnessTitle', () => {
  it('uses the product name for Macro slugs instead of title-casing them', () => {
    expect(harnessTitle('macro-inmem')).toBe('Macro Agent');
    expect(harnessTitle('in-memory')).toBe('Macro Agent');
    expect(harnessTitle('sandbox')).toBe('Macro Agent');
  });

  it('title-cases other slugs', () => {
    expect(harnessTitle(undefined)).toBe('Agent session');
  });
});

describe('sessionHarnessTitle', () => {
  it('uses the product name for Macro slugs', () => {
    expect(sessionHarnessTitle({ harness: 'macro-inmem' })).toBe('Macro Agent');
    expect(sessionHarnessTitle({ harness: 'in-memory' })).toBe('Macro Agent');
  });
});

describe('showsSessionHarness', () => {
  it('hides the Details row', () => {
    expect(showsSessionHarness({ harness: 'in-memory' })).toBe(false);
    expect(showsSessionHarness({ harness: 'macro-inmem' })).toBe(false);
    expect(showsSessionHarness({})).toBe(false);
  });
});
