import type { InputAttachmentData } from '@channel/Input/types';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@solidjs/testing-library';
import { createSignal, type JSX } from 'solid-js';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentsMode } from '../core/mode';
import {
  buildAgentRoster,
  type PersistedAgentLike,
  type RuntimeLike,
} from '../core/roster';
import { NEW_CONVERSATION_ATTACHMENTS_KEY } from '../primitives/composer-draft';
import { AgentPicker } from './AgentPicker';
import { NewChatPage } from './NewChatPage';

const mocks = vi.hoisted(() => ({
  userId: () => 'user' as string | undefined,
  touch: false,
  freePlan: false,
  modelsPending: false,
  tier: 'max',
  currentEffort: 'low',
  openSettings: vi.fn(),
  capabilitiesPending: false,
  attachments: [] as InputAttachmentData[],
  recentIds: [] as string[],
  recentUrls: [] as string[],
  preferredInmemModel: undefined as string | undefined,
  rememberInmemModel: vi.fn((id: string) => {
    mocks.preferredInmemModel = id;
  }),
  repositories: [] as { url: string; defaultBranch?: string }[],
}));
vi.mock('@core/mobile/isTouchDevice', () => ({
  isTouchDevice: () => mocks.touch,
}));
vi.mock('@core/util/upload', () => ({ uploadFile: vi.fn() }));
vi.mock('@channel/Input', async () => ({
  ...(await import('../../channel/Input/attachment-tracker')),
  uploadInputAttachments: vi.fn(),
}));
vi.mock('@core/context/user', () => ({
  useUserId: () => () => mocks.userId(),
}));
vi.mock('@core/constant/SettingsState', () => ({
  useSettingsState: () => ({ openSettings: mocks.openSettings }),
}));
vi.mock('@app/features/block-agent/context/recent-agent-selections', () => ({
  createRecentAgentSelections: () => ({
    ids: () => mocks.recentIds,
    remember: vi.fn(),
  }),
}));
vi.mock('../primitives/recent-repositories', () => ({
  createRecentRepositories: () => ({
    urls: () => mocks.recentUrls,
    remember: vi.fn(),
  }),
}));
vi.mock('../primitives/preferred-inmem-model', () => ({
  createPreferredInmemModel: () => ({
    model: () => mocks.preferredInmemModel,
    remember: mocks.rememberInmemModel,
  }),
}));
vi.mock('../queries/reachable-repositories', () => ({
  createReachableRepositories: () => ({
    repositories: () => mocks.repositories,
    loading: () => false,
    error: () => false,
    retry: vi.fn(),
  }),
}));
vi.mock('../queries/repository-branches', () => ({
  createRepositoryBranches: () => ({
    branches: () => ['main', 'develop', 'feature/home'],
    loading: () => false,
    error: () => false,
    retry: vi.fn(),
  }),
}));
vi.mock('../components/AgentGlyph', () => ({ AgentIcon: () => <span /> }));

vi.mock('@queries/auth/ai-billing', () => ({
  useAiBillingSummaryQuery: () => ({
    isSuccess: true,
    get data() {
      return { tier: mocks.tier };
    },
  }),
}));
vi.mock('@queries/agents/capabilities', () => ({
  useAgentCapabilitiesQuery: (
    target: () => { model?: string } | undefined
  ) => ({
    get isSuccess() {
      return !mocks.capabilitiesPending;
    },
    isFetching: false,
    get data() {
      if (target()?.model !== 'gpt-5') return { configOptions: [] };
      return {
        configOptions: [
          {
            id: 'cursor_effort',
            name: 'Effort',
            category: 'thought_level',
            type: 'select',
            currentValue: mocks.currentEffort,
            options: [
              { value: 'low', name: 'Low' },
              { value: 'ultra', name: 'Ultra' },
            ],
          },
        ],
      };
    },
  }),
}));

vi.mock('@queries/agents/models', () => ({
  useAgentModelsQuery: (
    target: () => { harness: string },
    enabled: () => boolean
  ) => ({
    get isSuccess() {
      return enabled() && !mocks.modelsPending;
    },
    get isPending() {
      return mocks.modelsPending;
    },
    get isError() {
      return false;
    },
    get data() {
      if (!enabled()) throw new Error('Pending resource must not be read');
      const harness = target().harness;
      if (harness === 'in-memory' && mocks.freePlan) {
        return {
          status: 'available',
          currentModel: 'google/gemini-3.8-flash',
          models: [{ id: 'google/gemini-3.8-flash', name: 'Gemini 3.8 Flash' }],
        };
      }
      return {
        status: 'available',
        currentModel: harness === 'cursor' ? 'cursor-default' : 'chat-default',
        models:
          harness === 'cursor'
            ? [
                { id: 'cursor-default', name: 'Cursor default' },
                { id: 'gpt-5', name: 'GPT-5' },
              ]
            : [
                { id: 'chat-default', name: 'Chat default' },
                { id: 'claude-sonnet-4', name: 'Sonnet 4' },
                {
                  id: 'anthropic/claude-sonnet-5-5',
                  name: 'anthropic/claude-sonnet-5-5',
                },
                { id: 'anthropic/claude-opus-5-5', name: 'Claude Opus 5.5' },
                { id: 'openai/gpt-5.6', name: 'GPT-5.6' },
                { id: 'openai/gpt-6-astra', name: 'GPT-6 Astra' },
                { id: 'google/gemini-3.8-flash', name: 'Gemini 3.8 Flash' },
                { id: 'fireworks/kimi-k3', name: 'Kimi K3' },
                { id: 'fireworks/glm-5p3', name: 'GLM 5.3' },
                {
                  id: 'fireworks/glm-5p3-flash',
                  name: 'GLM 5.3 Flash',
                },
                { id: 'fireworks/qwen3p8-max', name: 'Qwen 3.8 Max' },
                { id: 'fireworks/minimax-m3', name: 'MiniMax M3' },
                { id: 'cerebras/gpt-oss-120b', name: 'GPT OSS 120B' },
              ],
      };
    },
  }),
}));

// Keep the real picker and send wiring; substitute only the Lexical editor.
type ComposerProps = {
  corner: JSX.Element;
  selector: JSX.Element;
  drawer: JSX.Element;
  drawerOpen: boolean;
  draft: string;
  attachments: InputAttachmentData[];
  onDraftChange: (draft: string) => void;
  onSend: (prompt: string, attachments: InputAttachmentData[]) => void;
};
vi.mock('../components/ChatComposer', () => ({
  ChatComposer: (props: ComposerProps) => (
    <>
      {props.corner}
      {props.selector}
      <output data-testid="attachments">
        {props.attachments.map((a) => a.name).join(',')}
      </output>
      <div data-testid="drawer" hidden={!props.drawerOpen}>
        {props.drawer}
      </div>
      <input
        aria-label="Draft"
        value={props.draft}
        onInput={(event) => props.onDraftChange(event.currentTarget.value)}
      />
      <button
        onClick={() =>
          props.onSend(
            props.draft || (mocks.attachments.length ? '' : 'Prompt'),
            mocks.attachments
          )
        }
      >
        Send
      </button>
    </>
  ),
}));

beforeEach(() => {
  localStorage.clear();
  mocks.userId = () => 'user';
});

function page(
  _connected = true,
  agents: PersistedAgentLike[] = [],
  availabilityLoading = false,
  _runtimes: unknown[] = [],
  mode?: AgentsMode
) {
  const onStart = vi.fn();
  render(() => (
    <NewChatPage
      compact={mocks.touch}
      mode={mode}
      roster={buildAgentRoster({
        agents,
        macroDefaultModel: 'chat-default',
      })}
      rosterLoading={false}
      availabilityLoading={availabilityLoading}
      onStart={onStart}
      onOpenRoster={vi.fn()}
    />
  ));
  return onStart;
}
function openAgents() {
  const trigger = screen.getByRole('button', { name: 'Agent' });
  fireEvent.keyDown(trigger, { key: 'Enter' });
  const overview = screen.queryByRole('menuitem', { name: 'Choose model' });
  if (overview) fireEvent.keyDown(overview, { key: 'Enter' });
  return trigger;
}
async function selectAgent(name: RegExp) {
  const trigger = openAgents();
  const item = screen.getByRole('menuitem', { name });
  if (item.hasAttribute('aria-haspopup')) fireEvent.click(item);
  else fireEvent.keyDown(item, { key: 'Enter' });
  await waitFor(() =>
    expect(trigger.getAttribute('aria-expanded')).toBe('false')
  );
}
async function hoverAgent(name: string) {
  openAgents();
  // Chat mode's root menu has its own model search; Code mode's does not.
  const before = screen.queryAllByRole('textbox', {
    name: 'Search models',
  }).length;
  const event = new MouseEvent('pointermove', { bubbles: true });
  Object.defineProperty(event, 'pointerType', { value: 'mouse' });
  screen
    .getByRole('menuitem', { name: new RegExp(`^${name}`) })
    .dispatchEvent(event);
  await waitFor(() =>
    expect(
      screen.getAllByRole('textbox', { name: 'Search models' })
    ).toHaveLength(before + 1)
  );
  const search = screen
    .getAllByRole('textbox', { name: 'Search models' })
    .at(-1);
  if (!search) throw new Error('Agent model search did not open');
  return within(search.closest('[role="menu"]') as HTMLElement);
}

describe('agent-led new conversation', () => {
  let motionStyles: HTMLStyleElement;
  beforeEach(() => {
    mocks.capabilitiesPending = false;
    mocks.freePlan = false;
    mocks.modelsPending = false;
    mocks.tier = 'max';
    mocks.currentEffort = 'low';
    mocks.touch = false;
    mocks.attachments = [];
    mocks.recentIds = [];
    mocks.recentUrls = [];
    mocks.preferredInmemModel = undefined;
    mocks.repositories = [
      { url: 'https://github.com/macro-inc/macro', defaultBranch: 'develop' },
    ];
    vi.clearAllMocks();
    mocks.rememberInmemModel.mockImplementation((id: string) => {
      mocks.preferredInmemModel = id;
    });
    motionStyles = document.createElement('style');
    motionStyles.textContent =
      '[role="menu"], [data-corvu-drawer-content], [data-corvu-drawer-overlay] { animation-name: none; transition-duration: 0s; }';
    document.head.append(motionStyles);
    vi.stubGlobal('scrollTo', vi.fn());
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      }
    );
    Element.prototype.scrollTo = vi.fn();
  });
  afterEach(() => {
    cleanup();
    mocks.touch = false;
    motionStyles.remove();
    vi.unstubAllGlobals();
  });
  it.each([390, 1024])(
    'keeps the touch picker as a single sheet at %ipx',
    async (width) => {
      vi.stubGlobal('innerWidth', width);
      mocks.touch = true;
      page();
      fireEvent.click(screen.getByRole('button', { name: 'Agent' }));
      const sheet = await screen.findByRole('dialog');
      expect(
        within(sheet).getByRole('textbox', { name: 'Search agents and models' })
      ).toBeTruthy();
      expect(screen.queryByRole('menu')).toBeNull();
      expect(screen.queryByRole('group', { name: 'Agents' })).toBeNull();
      expect(screen.queryByRole('group', { name: 'Models' })).toBeNull();
      fireEvent.click(
        within(sheet).getByRole('button', { name: 'Sonnet 5.5' })
      );
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    }
  );
  it('filters models in the phone sheet and selects an in-memory model', async () => {
    mocks.touch = true;
    const send = page();
    fireEvent.click(screen.getByRole('button', { name: 'Agent' }));
    await screen.findByRole('textbox', { name: 'Search agents and models' });
    const frontier = within(screen.getByRole('group', { name: 'Suggested' }));
    expect(
      frontier.getAllByRole('button').map((item) => item.textContent?.trim())
    ).toEqual([
      'Opus 5.5',
      'Sonnet 5.5',
      'GPT-5.6',
      'GPT-6 Astra',
      'Gemini 3.8 Flash',
    ]);
    expect(screen.queryByRole('group', { name: 'Google' })).toBeNull();
    expect(screen.getByRole('button', { name: 'More models' })).toBeTruthy();
    expect(screen.queryByText('Recommended')).toBeNull();
    fireEvent.input(
      screen.getByRole('textbox', { name: 'Search agents and models' }),
      {
        target: { value: 'Sonnet 5.5' },
      }
    );
    expect(screen.queryByRole('button', { name: 'Chat default' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Sonnet 5.5' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        modelOverride: 'anthropic/claude-sonnet-5-5',
      })
    );
  });
  it('opens and focuses the mobile model search on the first tap', async () => {
    mocks.touch = true;
    page();
    const trigger = screen.getByRole('button', { name: 'Agent' });
    for (const type of ['pointerdown', 'mousedown']) {
      const event = new MouseEvent(type, { bubbles: true, cancelable: true });
      trigger.dispatchEvent(event);
      expect(event.defaultPrevented).toBe(true);
    }
    fireEvent.pointerUp(trigger);
    fireEvent.mouseUp(trigger);
    fireEvent.click(trigger);
    const search = await screen.findByRole('textbox', {
      name: 'Search agents and models',
    });
    await waitFor(() => expect(document.activeElement).toBe(search));
  });
  it('opens the full desktop catalog without search or a More step', async () => {
    page();
    for (let attempt = 0; attempt < 2; attempt++) {
      openAgents();
      expect(
        screen.queryByRole('textbox', { name: 'Search models' })
      ).toBeNull();
      expect(
        screen.queryByRole('menuitem', { name: 'More models' })
      ).toBeNull();
      expect(
        within(screen.getByRole('group', { name: 'Suggested' })).getByTitle(
          'Sonnet 5.5'
        )
      ).toBeTruthy();
      expect(screen.getByTitle('GLM 5.3 Flash')).toBeTruthy();
      fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
      await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    }
  });
  it('separates the desktop agent and model choices into named groups', () => {
    page();
    openAgents();
    const agents = screen.getByRole('group', { name: 'Agents' });
    const models = screen.getByRole('group', { name: 'Models' });
    expect(screen.queryByText('Your agents')).toBeNull();
    expect(screen.queryByText('Suggested')).toBeNull();
    const suggested = within(models).getByRole('group', { name: 'Suggested' });
    const anthropic = within(models).getByRole('group', { name: 'Anthropic' });
    expect(
      suggested.compareDocumentPosition(anthropic) &
        Node.DOCUMENT_POSITION_FOLLOWING
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

    expect(
      within(agents).getByRole('menuitem', { name: 'Create agent' })
    ).toBeTruthy();
    expect(
      within(models).queryByRole('textbox', { name: 'Search models' })
    ).toBeNull();
    expect(
      within(
        within(models).getByRole('group', { name: 'Suggested' })
      ).getByRole('menuitem', { name: 'Opus 5.5' })
    ).toBeTruthy();
  });
  it('flags a send before the model catalog loads as a model fallback', () => {
    mocks.modelsPending = true;
    const send = page();
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send).toHaveBeenCalledWith({
      onFailure: expect.any(Function),
      prompt: 'Prompt',
      botId: undefined,
      repoUrl: undefined,
      modelFallback: true,
    });
  });
  it('restores an unsent draft after the page remounts', () => {
    page();
    fireEvent.input(screen.getByRole('textbox', { name: 'Draft' }), {
      target: { value: 'Keep this prompt' },
    });
    cleanup();
    page();
    expect(
      (screen.getByRole('textbox', { name: 'Draft' }) as HTMLInputElement).value
    ).toBe('Keep this prompt');
  });
  it('uses Gemini for a free user starting a saved in-memory agent', async () => {
    mocks.freePlan = true;
    const send = page(true, [
      {
        bot: { id: 'saved-agent', name: 'Reviewer', handle: 'reviewer' },
        harness: 'in-memory',
        default_model: 'saved-default',
      },
    ]);
    await selectAgent(/Reviewer/);
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send).toHaveBeenCalledWith({
      onFailure: expect.any(Function),
      prompt: 'Prompt',
      botId: 'saved-agent',
      repoUrl: undefined,
      modelOverride: 'google/gemini-3.8-flash',
    });
  });
  it('groups the Macro catalog and selects direct models with readable names and icons', async () => {
    const send = page(true, [
      {
        bot: { id: 'saved-agent', name: 'Reviewer', handle: 'reviewer' },
        harness: 'in-memory',
        default_model: 'saved-default',
      },
    ]);
    openAgents();
    const models = within(screen.getByRole('group', { name: 'Models' }));
    expect(screen.getByRole('group', { name: 'Your agents' })).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'Coding agents' })).toBeNull();
    expect(screen.queryByRole('textbox', { name: 'Search models' })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: /More models/ })).toBeNull();
    expect(models.getByRole('group', { name: 'Z.ai' })).toBeTruthy();
    expect(models.getByTitle('GLM 5.3')).toBeTruthy();
    expect(models.getByTitle('GLM 5.3 Flash')).toBeTruthy();
    expect(screen.queryByRole('menuitem', { name: /Cursor/ })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: /^Macro/ })).toBeNull();
    const sonnet = within(
      models.getByRole('group', { name: 'Suggested' })
    ).getByTitle('Sonnet 5.5');
    expect(
      sonnet.querySelector('[data-ai-provider="anthropic"] svg')
    ).toBeTruthy();
    expect(screen.queryByText('anthropic/claude-sonnet-5-5')).toBeNull();
    sonnet.focus();
    fireEvent.keyDown(sonnet, { key: 'Enter' });
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    expect(mocks.rememberInmemModel).toHaveBeenCalledWith(
      'anthropic/claude-sonnet-5-5'
    );
    expect(screen.getByRole('button', { name: 'Agent' }).title).toBe(
      'Sonnet 5.5'
    );
    const trigger = screen.getByRole('button', { name: 'Agent' });
    expect(trigger.title).toBe('Sonnet 5.5');
    expect(
      trigger.querySelector('[data-ai-provider="anthropic"] svg')
    ).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Repository' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send).toHaveBeenLastCalledWith({
      onFailure: expect.any(Function),
      prompt: 'Prompt',
      botId: undefined,
      repoUrl: undefined,
      modelOverride: 'anthropic/claude-sonnet-5-5',
    });
    // Macro Models picks stick: a second send still uses the preferred model.
    expect(screen.getByRole('button', { name: 'Agent' }).title).toBe(
      'Sonnet 5.5'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send.mock.calls[1][0]).toMatchObject({
      modelOverride: 'anthropic/claude-sonnet-5-5',
    });
  });
  it('keeps unavailable Macro models disabled throughout the catalog', async () => {
    const roster = buildAgentRoster({
      agents: [],
      runtimes: [],
      cursorConnected: true,
      cursorNeedsConnection: false,
    });
    const macro = {
      ...roster[0],
      unavailableReason: 'Temporarily unavailable',
    };
    const onSelect = vi.fn();
    render(() => (
      <AgentPicker
        agents={[macro]}
        selected={macro}
        loading={false}
        onSelect={onSelect}
        onSelectEffort={onSelect}
        onConnect={vi.fn()}
        onCreate={vi.fn()}
      />
    ));
    openAgents();
    const sonnet = within(
      screen.getByRole('group', { name: 'Suggested' })
    ).getByTitle('Sonnet 5.5');
    expect(sonnet.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(sonnet);
    fireEvent.keyDown(sonnet, { key: 'Enter' });

    const flash = screen.getByTitle('GLM 5.3 Flash');
    expect(flash.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(flash);
    fireEvent.keyDown(flash, { key: 'Enter' });
    expect(onSelect).not.toHaveBeenCalled();
  });
  it('restores a preferred Macro model on a fresh composer', () => {
    mocks.preferredInmemModel = 'anthropic/claude-sonnet-5-5';
    page();
    expect(screen.getByRole('button', { name: 'Agent' }).title).toBe(
      'Sonnet 5.5'
    );
  });
  it('uses the free catalog default instead of a persisted paid model', async () => {
    mocks.freePlan = true;
    mocks.preferredInmemModel = 'anthropic/claude-sonnet-5-5';
    const send = page();
    expect(screen.getByRole('button', { name: 'Agent' }).title).toBe(
      'Gemini 3.8 Flash'
    );
    openAgents();
    await waitFor(() => expect(screen.getByRole('menu')).toBeTruthy());
    expect(screen.queryByTitle('Sonnet 5.5')).toBeNull();
    expect(
      within(screen.getByRole('group', { name: 'Suggested' })).getByTitle(
        'Gemini 3.8 Flash'
      )
    ).toBeTruthy();
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        modelOverride: 'google/gemini-3.8-flash',
      })
    );
  });
});

it('starts a conversation with an uploaded image and no text', () => {
  mocks.attachments = [
    {
      id: 'sfs-image',
      name: 'pasted.png',
      kind: 'image',
      mimeType: 'image/png',
      size: 123,
    },
  ];
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  );
  const start = page();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  expect(start).toHaveBeenCalledWith(
    expect.objectContaining({
      onFailure: expect.any(Function),
      prompt: '',
      attachments: [
        {
          uri: expect.stringContaining('sfs-image'),
          name: 'pasted.png',
          mimeType: 'image/png',
          size: 123,
        },
      ],
    })
  );
});

it('restores a text-only draft when startup configuration fails', () => {
  mocks.attachments = [];
  const start = page();
  fireEvent.input(screen.getByRole('textbox', { name: 'Draft' }), {
    target: { value: 'Keep my unsent prompt' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  fireEvent.input(screen.getByRole('textbox', { name: 'Draft' }), {
    target: { value: '' },
  });
  start.mock.calls[0][0].onFailure();
  expect(
    (screen.getByRole('textbox', { name: 'Draft' }) as HTMLInputElement).value
  ).toBe('Keep my unsent prompt');
});

it('retains uploaded attachments until first delivery is accepted', () => {
  mocks.attachments = [
    {
      id: 'retained-file',
      name: 'keep.txt',
      kind: 'document',
      mimeType: 'text/plain',
      size: 12,
    },
  ];
  localStorage.setItem(
    `${NEW_CONVERSATION_ATTACHMENTS_KEY}:user`,
    JSON.stringify(mocks.attachments)
  );
  const start = page();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  expect(
    JSON.parse(
      localStorage.getItem(`${NEW_CONVERSATION_ATTACHMENTS_KEY}:user`)!
    )
  ).toEqual(mocks.attachments);
  start.mock.calls[0][0].onFailure();
  expect(
    JSON.parse(
      localStorage.getItem(`${NEW_CONVERSATION_ATTACHMENTS_KEY}:user`)!
    )
  ).toEqual(mocks.attachments);
  start.mock.calls[0][0].onDelivered();
  expect(
    JSON.parse(
      localStorage.getItem(`${NEW_CONVERSATION_ATTACHMENTS_KEY}:user`)!
    )
  ).toEqual([]);
});

it('isolates drafts and attachment projections as identity arrives and changes', () => {
  const [identity, setIdentity] = createSignal<string>();
  mocks.userId = identity;
  const base = 'attachment-tracker-agents-new-conversation-persist-v0';
  const attachment = { id: 'private', name: 'alice-only.txt', kind: 'file' };
  localStorage.setItem(`${base}:`, JSON.stringify([attachment]));
  localStorage.setItem(`${base}:alice`, JSON.stringify([attachment]));
  page();
  expect(screen.getByTestId('attachments').textContent).toBe('');
  fireEvent.input(screen.getByLabelText('Draft'), {
    target: { value: 'anonymous' },
  });
  setIdentity('alice');
  expect((screen.getByLabelText('Draft') as HTMLInputElement).value).toBe('');
  expect(screen.getByTestId('attachments').textContent).toBe('alice-only.txt');
  fireEvent.input(screen.getByLabelText('Draft'), {
    target: { value: 'Alice private' },
  });
  setIdentity('bob');
  expect((screen.getByLabelText('Draft') as HTMLInputElement).value).toBe('');
  expect(screen.getByTestId('attachments').textContent).toBe('');
  setIdentity('alice');
  expect((screen.getByLabelText('Draft') as HTMLInputElement).value).toBe(
    'Alice private'
  );
});

it('does not restore a failed first prompt into another account', () => {
  const [identity, setIdentity] = createSignal<string | undefined>('alice');
  mocks.userId = identity;
  mocks.attachments = [];
  const start = page();
  fireEvent.input(screen.getByLabelText('Draft'), {
    target: { value: 'Alice prompt' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  setIdentity('bob');
  start.mock.calls[0][0].onFailure();
  expect((screen.getByLabelText('Draft') as HTMLInputElement).value).toBe('');
});

it('clears delivered attachments after switching away and back to the submitting account', () => {
  const [identity, setIdentity] = createSignal<string | undefined>('alice');
  mocks.userId = identity;
  mocks.attachments = [{ id: 'sent-file', name: 'sent.txt', kind: 'document' }];
  localStorage.setItem(
    `${NEW_CONVERSATION_ATTACHMENTS_KEY}:alice`,
    JSON.stringify(mocks.attachments)
  );
  const start = page();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  setIdentity('bob');
  setIdentity('alice');
  expect(screen.getByTestId('attachments').textContent).toBe('sent.txt');
  start.mock.calls[0][0].onDelivered();
  expect(screen.getByTestId('attachments').textContent).toBe('');
});
