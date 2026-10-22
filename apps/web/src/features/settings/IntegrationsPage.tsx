import { ENABLE_EMAIL } from '@core/constant/featureFlags';
import { useSettingsState } from '@core/constant/SettingsState';
import { usePipedreamMcpFlag } from '@core/pipedream/flag';
import { Show, Suspense } from 'solid-js';
import { IntegrationsView } from './connections/integrations-view';
import { EmailCard } from './Email';
import { GitHubCard } from './GitHub';
import { GranolaCard } from './Granola';
import { IntegrationsSection } from './Integrations';
import { SettingsPage } from './primitives';

/** Personal accounts and agent tools share one Integrations settings page. */
export function IntegrationsPage() {
  const pipedreamMcp = usePipedreamMcpFlag();
  const { openSettings } = useSettingsState();
  return (
    <Show
      when={pipedreamMcp()}
      fallback={
        <SettingsPage title="Integrations">
          <AccountConnections />
          <Suspense>
            <IntegrationsSection />
          </Suspense>
        </SettingsPage>
      }
    >
      <IntegrationsView onOpenMacroMcp={() => openSettings('Agent')}>
        <AccountConnections />
      </IntegrationsView>
    </Show>
  );
}

function AccountConnections() {
  const { openSettings } = useSettingsState();
  return (
    <section
      data-settings-target="accounts"
      tabIndex={-1}
      class="pb-5 outline-none"
    >
      <h2 class="mb-4 text-lg font-medium text-ink">Core Integrations</h2>
      <div class="flex flex-col gap-3">
        <Show when={ENABLE_EMAIL}>
          <Suspense>
            <EmailCard onOpenSettings={() => openSettings('Email')} />
          </Suspense>
        </Show>
        <Suspense>
          <GitHubCard />
        </Suspense>
        <Suspense>
          <GranolaCard />
        </Suspense>
      </div>
    </section>
  );
}
