import { promptActionOf } from '@app/features/block-agent/component/prompt-action';
import { createRecentAgentSelections } from '@app/features/block-agent/context/recent-agent-selections';
import {
  createInputAttachmentTracker,
  type InputAttachmentData,
  uploadInputAttachments,
} from '@channel/Input';
import { FloatRegionOrInline } from '@components/app/mobile/float-regions/FloatRegion';
import { SpeedToggle } from '@core/component/AI/component/input/SpeedToggle';
import {
  acceleratedSpeed,
  speedForModel,
} from '@core/component/AI/constant/speed';
import { fastModeEnabled } from '@core/component/AI/signal/speed';
import { ComposerGreeting } from '@core/component/ComposerGreeting';
import { useUserId } from '@core/context/user';
import { fireAndForget } from '@core/util/fireAndForget';
import { uploadFile } from '@core/util/upload';
import { useAgentCapabilitiesQuery } from '@queries/agents/capabilities';
import { useAiBillingSummaryQuery } from '@queries/auth/ai-billing';
import type { PromptAttachment } from '@service-agent-harness/generated/schemas';
import { tourTarget } from '@ui/components/Tour';
import { createMemo, createSignal, Show } from 'solid-js';
import { EffortSelector } from '../../block-agent/component/EffortSelector';
import {
  type EffortChoice,
  effortConfigOption,
  effortLabel,
} from '../../block-agent/state/session-config';
import { ChatComposer } from '../components/ChatComposer';
import type { AgentKind } from '../core/agent-kind';
import type { AgentsMode } from '../core/mode';
import { MACRO_PERSONA_ID, type RosterAgent } from '../core/roster';
import {
  createPersistedComposerDraft,
  NEW_CONVERSATION_ATTACHMENTS_KEY,
} from '../primitives/composer-draft';
import { createPreferredInmemModel } from '../primitives/preferred-inmem-model';
import { createComposerModels } from '../queries/composer-models';
import { AGENTS_TOUR } from '../tour';
import { AgentPicker } from './AgentPicker';

/** What the composer hands the workspace to start a session with. */
export type StartConversation = {
  prompt: string;
  attachments?: PromptAttachment[];
  /** Persisted or first-party bot to run; omitted for Macro's default. */
  botId?: string;
  repoUrl?: string;
  repoBranch?: string;
  modelOverride?: string;
  /** The catalog was still loading, so an in-memory agent runs its persona default. */
  modelFallback?: boolean;
  effortOverride?: { configId: string; value: string };
  speedOverride?: { configId: string; value: string };
  onDelivered?: () => void;
  onFailure?: () => void;
};

/** One agent choice determines the default model and configuration. */
export function NewChatPage(props: {
  compact?: boolean;
  active?: boolean;
  workspaceId?: string;
  draft?: string;
  onDraftChange?: (draft: string) => void;
  autoFocus?: boolean;
  registerFocus?: (focus: () => void) => void;
  roster: RosterAgent[];
  mode?: AgentsMode;
  onModeChange?: (mode: AgentsMode) => void;
  rosterLoading: boolean;
  availabilityLoading?: boolean;
  onStart: (start: StartConversation) => void;
  /** Opens the roster page. */
  onOpenRoster: (kind: AgentKind) => void;
}) {
  const userId = useUserId();
  const billing = useAiBillingSummaryQuery();
  const recentAgents = createRecentAgentSelections(userId());
  const preferredInmem = createPreferredInmemModel(userId());
  const options = () => props.roster;
  const [agentId, setAgentId] = createSignal<string>();
  const [modelOverride, setModelOverride] = createSignal<string>();
  const [effortPickerOpen, setEffortPickerOpen] = createSignal(false);
  const persistedDraft = createPersistedComposerDraft(undefined, userId);
  const draft = () => props.draft ?? persistedDraft.draft();
  const setDraft = (text: string) =>
    props.onDraftChange
      ? props.onDraftChange(text)
      : persistedDraft.setDraft(text);
  const recentAgentId = () => {
    const settling = props.rosterLoading || props.availabilityLoading;
    return recentAgents
      .ids()
      .find((id) =>
        options().some(
          (agent) => agent.id === id && (settling || !agent.unavailableReason)
        )
      );
  };
  const selected = createMemo(() => {
    for (const wanted of [agentId(), recentAgentId(), MACRO_PERSONA_ID]) {
      const agent = options().find((option) => option.id === wanted);
      if (agent) return agent;
    }
    return options().find((agent) => !agent.unavailableReason) ?? options()[0];
  });
  const selectedCatalog = createComposerModels(selected);
  /** In-memory choices come from the owner's catalog; other runtimes keep their rules. */
  const composerModelOverride = () => {
    const agent = selected();
    if (agent?.harness !== 'macro-inmem' && agent?.harness !== 'in-memory')
      return modelOverride();
    const preferred =
      modelOverride() ??
      (agent.id === MACRO_PERSONA_ID
        ? preferredInmem.model()
        : agent.defaultModel);
    return selectedCatalog.models().some((option) => option.id === preferred)
      ? preferred
      : selectedCatalog.currentModel();
  };
  const capabilityTarget = () => {
    const agent = selected();
    const harness =
      agent?.harness === 'macro-inmem' ? 'in-memory' : agent?.harness;
    if (harness !== 'in-memory') return undefined;
    return {
      harness,
      model: composerModelOverride(),
    } as const;
  };
  const capabilities = useAgentCapabilitiesQuery(capabilityTarget);
  const effort = () =>
    effortConfigOption(
      capabilities.isSuccess ? capabilities.data.configOptions : []
    );
  const [effortSelection, setEffortSelection] = createSignal<{
    target: string;
    configId: string;
    value: string;
    name: string;
  }>();
  const selectedEffort = () => {
    const selection = effortSelection();
    return selection?.target === JSON.stringify(capabilityTarget())
      ? selection
      : undefined;
  };
  const effortOverride = () => {
    const config = effort();
    const minimum =
      billing.isSuccess &&
      (billing.data.tier === 'free' || billing.data.tier === 'premium')
        ? config?.options[0]
        : undefined;
    const selection =
      selectedEffort() ??
      (minimum && config
        ? { configId: config.id, value: minimum.value, name: minimum.name }
        : undefined);
    return selection
      ? { configId: selection.configId, value: selection.value }
      : undefined;
  };
  const blocked = () => {
    const agent = selected();
    return agent ? agent.unavailableReason : 'Choose an agent to start';
  };

  const attachmentTracker = createMemo(() => {
    const identity = userId();
    return createInputAttachmentTracker({
      persistenceKey:
        !props.onDraftChange && identity
          ? `${NEW_CONVERSATION_ATTACHMENTS_KEY}:${encodeURIComponent(identity)}`
          : undefined,
    });
  });
  const attachFiles = (files: File[]) =>
    fireAndForget(
      uploadInputAttachments({
        files,
        tracker: attachmentTracker(),
        uploadFile: (file) =>
          uploadFile(file, 'static', { hideProgressIndicator: true }),
      }),
      'agents-view'
    );

  const send = (prompt: string, attachments: InputAttachmentData[]) => {
    const persona = selected();
    if (
      (!prompt.trim() && attachments.length === 0) ||
      !persona ||
      blocked() ||
      attachmentTracker().hasPending()
    )
      return;
    recentAgents.remember(persona.id);
    const model = composerModelOverride();
    const inMemory =
      persona.harness === 'macro-inmem' || persona.harness === 'in-memory';
    const submittedTracker = attachmentTracker();
    const submittedUserId = userId();
    props.onStart({
      prompt,
      onFailure: () => {
        if (userId() === submittedUserId && !draft()) setDraft(prompt);
      },
      ...(attachments.length > 0
        ? {
            attachments: promptActionOf(prompt, attachments).attachments,
            onDelivered: () => {
              const tracker =
                userId() === submittedUserId
                  ? attachmentTracker()
                  : submittedTracker;
              for (const attachment of attachments) {
                tracker.removeAttachment(attachment.id);
              }
            },
          }
        : {}),
      botId: persona.botId,
      ...(model ? { modelOverride: model } : {}),
      ...(inMemory && !model && selectedCatalog.pending()
        ? { modelFallback: true }
        : {}),
      effortOverride: effortOverride(),
      ...(model && inMemory && acceleratedSpeed(model)
        ? {
            speedOverride: {
              configId: 'speed',
              value: speedForModel(model, fastModeEnabled()),
            },
          }
        : {}),
    });
    setModelOverride(undefined);
    setEffortSelection(undefined);
  };

  const selectAgent = (
    agent: RosterAgent,
    model?: string,
    selection?: EffortChoice
  ) => {
    setAgentId(agent.id);
    if (agent.id === MACRO_PERSONA_ID) {
      if (model) preferredInmem.remember(model);
      setModelOverride(model);
    } else {
      setModelOverride(model);
    }
    setEffortSelection(
      selection
        ? { ...selection, target: JSON.stringify(capabilityTarget()) }
        : undefined
    );
  };

  let focusInput: (() => void) | undefined;
  const restoreInputFocus = (event: Event) => {
    event.preventDefault();
    focusInput?.();
  };
  const agentSelector = () => (
    <AgentPicker
      agents={options()}
      selected={selected()}
      modelOverride={composerModelOverride()}
      loading={props.rosterLoading}
      effortLabel={
        selectedEffort()?.name ?? effortLabel(effort(), effortOverride()?.value)
      }
      effortSelection={effortOverride()}
      onCloseAutoFocus={restoreInputFocus}
      onSelect={selectAgent}
      onSelectEffort={selectAgent}
      onCreate={() => props.onOpenRoster('agent')}
    />
  );

  const composer = () => (
    <ChatComposer
      autoFocus={props.autoFocus}
      collapseOnBlur={props.compact}
      controlsOpen={effortPickerOpen()}
      registerFocus={(focus) => {
        focusInput = focus;
        props.registerFocus?.(focus);
      }}
      draft={draft()}
      onDraftChange={setDraft}
      blockedReason={blocked()}
      selector={
        <div class="flex min-w-0 items-center gap-1">
          {agentSelector()}
          <EffortSelector
            onCloseAutoFocus={restoreInputFocus}
            onOpenChange={setEffortPickerOpen}
            config={effort()}
            value={effortOverride()?.value}
            disabled={Boolean(blocked())}
            onChange={(choice) =>
              setEffortSelection({
                ...choice,
                target: JSON.stringify(capabilityTarget()),
              })
            }
          />
          <Show
            when={['macro-inmem', 'in-memory'].includes(
              selected()?.harness ?? ''
            )}
          >
            <SpeedToggle model={composerModelOverride() ?? ''} />
          </Show>
        </div>
      }
      onSend={send}
      supportsImages={
        selectedCatalog
          .models()
          .find((model) => model.id === composerModelOverride())?.supportsImages
      }
      attachments={attachmentTracker().attachments()}
      onAttachFiles={attachFiles}
      onRemoveAttachment={(attachment) =>
        attachmentTracker().removeAttachment(attachment.id)
      }
    />
  );

  return (
    <Show when={!props.compact} fallback={composer()}>
      <section class="page newchat" data-active aria-label="New conversation">
        <div ref={tourTarget(AGENTS_TOUR.composer)} class="col">
          <div class="greeting">
            <ComposerGreeting>
              <span class="greeting-text">What should we work on?</span>
            </ComposerGreeting>
          </div>
          <FloatRegionOrInline
            region="accessory"
            active={() => props.active !== false}
          >
            <div
              data-agent-new-composer={props.workspaceId}
              class="agents-view-portal touch:px-(--mobile-chrome-gutter) pointer-events-auto min-w-0"
            >
              {composer()}
            </div>
          </FloatRegionOrInline>
        </div>
      </section>
    </Show>
  );
}
