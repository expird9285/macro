/** @vitest-environment jsdom */
import { cleanup, render, screen } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import { IntegrationsPage } from './IntegrationsPage';

const openSettings = vi.fn();
vi.mock('@core/constant/SettingsState', () => ({
  useSettingsState: () => ({ openSettings }),
}));
vi.mock('@core/constant/featureFlags', () => ({ ENABLE_EMAIL: true }));
vi.mock('@core/pipedream/flag', () => ({
  usePipedreamMcpFlag: () => () => true,
}));
vi.mock('./Email', () => ({ EmailCard: () => <div>Email</div> }));
vi.mock('./Granola', () => ({ GranolaCard: () => <div>Granola</div> }));
vi.mock('./GitHub', () => ({ GitHubCard: () => <div>GitHub</div> }));
vi.mock('./Integrations', () => ({ IntegrationsSection: () => null }));
vi.mock('./connections/integrations-view', () => ({
  IntegrationsView: () => (
    <div>
      <h1>Integrations</h1>
    </div>
  ),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

it('renders Integrations view', () => {
  render(() => <IntegrationsPage />);
  expect(screen.getByRole('heading', { name: 'Integrations' })).toBeTruthy();
});
