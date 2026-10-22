/**
 * @vitest-environment jsdom
 */

import { cleanup, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AgentSettings } from './AgentSettings';

vi.mock('./Agents', () => ({
  Agents: () => (
    <div>
      <h1>Agents page</h1>
    </div>
  ),
}));

afterEach(() => {
  cleanup();
});

describe('AgentSettings', () => {
  it('renders Agents page', () => {
    render(() => <AgentSettings />);
    expect(screen.getByRole('heading', { name: 'Agents page' })).toBeTruthy();
  });
});
