/**
 * The agent block's side-panel sections: session details, plan, activity, and references.
 */

import { SidePanel } from '@components/app/side-panel';
import { EntityMetadata } from '@components/app/side-panel/EntityMetadata';
import { ModelIcon } from '@core/component/AI/component/ProviderIcon';
import { References } from '@core/component/References';
import { useAttachmentReferencesQuery } from '@queries/storage/attachment-references';
import { createMemo, Show, Suspense } from 'solid-js';
import { useAgentSession } from '../../context/AgentSessionContext';
import { sessionStatus } from '../../state/session-status';
import { activityCounts, latestPlan } from '../../state/session-summary';
import { CountSummary, SessionStatusPill, TodoList } from '../../ui';
import { AgentPullRequestChip } from '../AgentPullRequestChip';
import { AgentTaskChip } from '../AgentTaskChip';
import { modelDisplayName } from '../compose-agent-session-options';

export function AgentSidePanelSections() {
  const { sessionId, session, bot, metadata, messages } = useAgentSession();

  const plan = createMemo(() => latestPlan(messages()));
  const activity = createMemo(() => activityCounts(messages()));

  return (
    <>
      <SidePanel.Footer>
        <EntityMetadata
          ownerId={session()?.ownerId}
          createdAt={session()?.createdAt}
          updatedAt={session()?.modifiedAt}
        />
      </SidePanel.Footer>
      <SidePanel.Section id="session" title="Session" defaultOpen order={10}>
        <SidePanel.Grid>
          <SidePanel.Row label="Status">
            <SessionStatusPill status={sessionStatus(metadata())} />
          </SidePanel.Row>
          <Show when={bot()?.name}>
            {(name) => (
              <SidePanel.Row label="Agent">
                <SidePanel.Pill>
                  <span class="truncate">{name()}</span>
                </SidePanel.Pill>
              </SidePanel.Row>
            )}
          </Show>
          <Show when={metadata()?.model ?? session()?.model}>
            {(model) => (
              <SidePanel.Row label="Model">
                <SidePanel.Pill>
                  <ModelIcon model={model()} class="size-3" />
                  <span class="truncate">
                    {modelDisplayName(
                      model(),
                      metadata()?.supportedModels ?? []
                    )}
                  </span>
                </SidePanel.Pill>
              </SidePanel.Row>
            )}
          </Show>
          <Show when={session()?.taskId}>
            {(taskId) => (
              <SidePanel.Row label="Task">
                <AgentTaskChip taskId={taskId()} />
              </SidePanel.Row>
            )}
          </Show>
          <Show when={session()?.pullRequestUrl}>
            {(url) => (
              <SidePanel.Row label="Pull request">
                <AgentPullRequestChip url={url()} />
              </SidePanel.Row>
            )}
          </Show>
        </SidePanel.Grid>
      </SidePanel.Section>

      <Show when={plan()}>
        {(entries) => (
          <SidePanel.Section id="plan" title="Plan" defaultOpen order={15}>
            <TodoList
              todos={entries().map((entry) => ({
                content: entry.content,
                status: entry.status,
              }))}
            />
          </SidePanel.Section>
        )}
      </Show>

      <Show when={activity().some((item) => item.count > 0)}>
        <SidePanel.Section id="activity" title="Activity" order={30}>
          <div class="text-xs text-ink-muted">
            <CountSummary items={activity()} />
          </div>
        </SidePanel.Section>
      </Show>

      <ReferencesSectionConditional sessionId={sessionId()} />
    </>
  );
}

/**
 * Where this session is referenced: channel messages that mention or attach
 * it, and documents that mention it. Same section the markdown, email, and
 * call blocks show; hidden until at least one reference exists.
 */
function ReferencesSectionConditional(props: { sessionId?: string }) {
  const references = useAttachmentReferencesQuery(
    () => props.sessionId,
    () => 'agent_session'
  );

  // Gate the resource read on status so a pending query never suspends the
  // enclosing block while the section is hidden anyway.
  const count = () => (references.isSuccess ? references.data.length : 0);

  return (
    <Show when={count() > 0 ? props.sessionId : undefined}>
      {(sessionId) => (
        <SidePanel.Section
          id="references"
          title={<SidePanel.CountTitle label="References" count={count()} />}
          order={40}
        >
          <Suspense fallback={<SidePanel.Loading />}>
            <div class="text-xs">
              <References documentId={sessionId()} entityType="agent_session" />
            </div>
          </Suspense>
        </SidePanel.Section>
      )}
    </Show>
  );
}
