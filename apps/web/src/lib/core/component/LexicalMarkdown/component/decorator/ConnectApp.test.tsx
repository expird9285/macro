import { fireEvent, render, screen } from '@solidjs/testing-library';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  openSettings: vi.fn(),
  openAgentsPage: vi.fn(),
  requestConnectApp: vi.fn(),
  pipedreamSlugs: new Set<string>(),
}));

vi.mock('@app/features/agents-view/primitives/open-page', () => ({
  openAgentsPage: mocks.openAgentsPage,
}));
vi.mock('@components/app/split-layout/layout', () => ({
  useSplitLayout: () => ({}),
}));
vi.mock('@core/constant/SettingsState', () => ({
  useSettingsState: () => ({ openSettings: mocks.openSettings }),
}));
vi.mock('@core/pipedream/pendingConnect', () => ({
  requestConnectApp: mocks.requestConnectApp,
}));
vi.mock('@core/pipedream/ConnectorIcon', () => ({
  PipedreamConnectorIcon: () => <span data-pipedream-icon />,
}));
vi.mock('@queries/pipedream-connectors', () => ({
  usePipedreamConnectedSlugs: () => ({
    ready: () => true,
    slugs: () => mocks.pipedreamSlugs,
  }),
}));
// Reached through LexicalWrapperContext; the plugin barrel is far heavier
// than the context object this chip reads its selection from.
vi.mock('../../plugins', () => ({}));

import { AppConnectionContext } from '@core/pipedream/connection-context';
import { ConnectApp } from './ConnectApp';

beforeEach(() => {
  vi.clearAllMocks();
  mocks.pipedreamSlugs.clear();
});

describe('connect-app chip', () => {
  it('connects in place when an agent session owns the continuation', () => {
    const connect = vi.fn(async () => {});
    render(() => (
      <AppConnectionContext.Provider value={{ connect, disabled: () => false }}>
        <ConnectApp
          appSlug="linear"
          name="Linear"
          target="connections"
          key="node"
          theme={{}}
        />
      </AppConnectionContext.Provider>
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Connect Linear' }));
    expect(connect).toHaveBeenCalledExactlyOnceWith({
      appSlug: 'linear',
      name: 'Linear',
    });
    expect(mocks.openAgentsPage).not.toHaveBeenCalled();
    expect(mocks.requestConnectApp).not.toHaveBeenCalled();
  });

  it('disables connection while the agent session cannot safely continue', () => {
    const connect = vi.fn(async () => {});
    render(() => (
      <AppConnectionContext.Provider value={{ connect, disabled: () => true }}>
        <ConnectApp
          appSlug="linear"
          name="Linear"
          target="connections"
          key="node"
          theme={{}}
        />
      </AppConnectionContext.Provider>
    ));
    expect(
      screen
        .getByRole('button', { name: 'Connect Linear' })
        .hasAttribute('disabled')
    ).toBe(true);
    expect(connect).not.toHaveBeenCalled();
  });

  it('sends a Pipedream chip to Connections with the app queued', () => {
    render(() => (
      <ConnectApp
        appSlug="linear"
        name="Linear"
        target="connections"
        key="node"
        theme={{}}
      />
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Connect Linear' }));
    expect(mocks.requestConnectApp).toHaveBeenCalledWith('linear');
    expect(mocks.openAgentsPage).toHaveBeenCalledWith({}, 'connections');
    expect(mocks.openSettings).not.toHaveBeenCalled();
  });

  it('renders nothing for unsupported harness target', () => {
    const { container } = render(() => (
      <ConnectApp
        appSlug="cursor"
        name="Cursor"
        target="harness"
        key="node"
        theme={{}}
      />
    ));
    expect(container.textContent).toBe('');
  });
});
