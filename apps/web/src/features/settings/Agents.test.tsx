/**
 * @vitest-environment jsdom
 */

import { Model } from '@core/component/AI/constant/model';
import { useAgentModelsQueries } from '@queries/agents/models';
import type { LoadAgentModelsResponse } from '@service-agent-harness/generated/schemas';
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@solidjs/testing-library';
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from '@tanstack/solid-query';
import { Suspense } from 'solid-js';
import { createStore } from 'solid-js/store';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { Agents } from './Agents';
import { chooseSelectOption, selectOptions } from './tests/select-helpers';

const agentsPageProps = {
  onCursor: vi.fn(),
  onSetUpRuntime: vi.fn(),
  onManageRuntimes: vi.fn(),
};

// The real Lexical surface has its own Markdown round-trip suite.
vi.mock('./components/instructions-editor', () => ({
  AgentInstructionsEditor: (props: {
    markdown: string;
    disabled?: boolean;
    onChange: (markdown: string) => void;
  }) => (
    <textarea
      aria-label="Instructions"
      disabled={props.disabled}
      value={props.markdown}
      onInput={(event) => props.onChange(event.currentTarget.value)}
    />
  ),
}));

const [searchParams, updateSearchParams] = createStore<{
  createAgent?: string;
}>({});
const setSearchParams = vi.fn((next: { createAgent?: string }) =>
  updateSearchParams(next)
);

vi.mock('@solidjs/router', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@solidjs/router')>()),
  useSearchParams: () => [searchParams, setSearchParams],
}));

const agentMocks = vi.hoisted(() => ({
  query: {
    isSuccess: true,
    isPending: false,
    data: [] as unknown[],
    isError: false,
  },
  create: vi.fn(),
  delete: vi.fn(),
  update: vi.fn(),
  toastSuccess: vi.fn(),
  toastFailure: vi.fn(),
  currentUserId: 'macro|user@example.com',
  currentTeam: { team: { id: 'team-1' } } as { team: { id: string } } | null,
  isTeamOwner: false,
}));

const avatarMocks = vi.hoisted(() => ({ upload: vi.fn() }));
vi.mock('@queries/agents/avatar', () => ({
  useUploadAgentAvatarMutation: () => ({ mutateAsync: avatarMocks.upload }),
}));

const modelMocks = vi.hoisted(() => ({
  queries: {} as Record<
    string,
    {
      data?: {
        status: 'available' | 'unsupported';
        currentModel?: string | null;
        models: { id: string; name: string; description?: string | null }[];
      };
      isPending: boolean;
      isError: boolean;
      isSuccess: boolean;
      refetch: ReturnType<typeof vi.fn>;
    }
  >,
}));

function modelTargetKey(target: {
  harness: string;
  harnessId?: string;
}): string {
  return `${target.harness}:${target.harnessId ?? ''}`;
}

function successfulModels(
  models: { id: string; name: string }[],
  currentModel = models[0]?.id
) {
  return {
    data: {
      status: 'available' as const,
      currentModel,
      models,
    },
    isPending: false,
    isError: false,
    isSuccess: true,
    refetch: vi.fn(),
  };
}

vi.mock('@queries/agents/models', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('@queries/agents/models')>();
  return {
    ...actual,
    useAgentModelsQueries: vi.fn(
      (targets: () => { harness: string; harnessId?: string }[]) =>
        targets().map(
          (target) =>
            modelMocks.queries[modelTargetKey(target)] ?? {
              isPending: true,
              isError: false,
              isSuccess: false,
              refetch: vi.fn(),
            }
        )
    ),
  };
});

vi.mock('@queries/agents/agents', () => ({
  useAgentsQuery: () => agentMocks.query,
  useCreateAgentMutation: () => ({
    mutateAsync: agentMocks.create,
    isPending: false,
  }),
  useDeleteAgentMutation: () => ({
    mutateAsync: agentMocks.delete,
    isPending: false,
  }),
  useUpdateAgentMutation: () => ({
    mutateAsync: agentMocks.update,
    isPending: false,
  }),
}));

vi.mock('@queries/team/teams', () => ({
  useCurrentTeamQuery: () => ({
    data: agentMocks.currentTeam,
    isSuccess: true,
  }),
  useIsTeamOwner: () => () => agentMocks.isTeamOwner,
}));

vi.mock('@core/context/user', () => ({
  useUserId: () => () => agentMocks.currentUserId,
}));

vi.mock('@core/component/Toast/Toast', () => ({
  toast: {
    success: agentMocks.toastSuccess,
    failure: agentMocks.toastFailure,
  },
}));

vi.mock('@core/context/channels', () => ({
  useChannelsContext: () => ({
    channels: () => [
      { id: 'channel-general', name: 'general', channel_type: 'private' },
      {
        id: 'channel-engineering',
        name: 'engineering',
        channel_type: 'team',
      },
    ],
  }),
}));

const pipedreamMocks = vi.hoisted(() => ({
  enabled: true,
  connected: [] as string[],
  connect: vi.fn(),
  catalog: [
    {
      app_slug: 'linear',
      display_name: 'Linear',
      description: 'Issue tracking',
      icon_url: null,
    },
    {
      app_slug: 'notion',
      display_name: 'Notion',
      description: 'Docs and wikis',
      icon_url: null,
    },
  ],
}));

vi.mock('@core/pipedream/flag', () => ({
  usePipedreamMcpFlag: () => () => pipedreamMocks.enabled,
}));

vi.mock('@queries/pipedream-connectors', () => ({
  usePipedreamConnectedSlugs: () => ({
    slugs: () => new Set(pipedreamMocks.connected),
    ready: () => true,
  }),
  usePipedreamCatalogQuery: (search: () => string) => ({
    get data() {
      const term = search().trim().toLowerCase();
      return {
        pages: [
          {
            servers: pipedreamMocks.catalog.filter((entry) =>
              entry.display_name.toLowerCase().includes(term)
            ),
            next_cursor: null,
          },
        ],
        pageParams: [undefined],
      };
    },
    isFetching: false,
    isSuccess: true,
    isFetchingNextPage: false,
    hasNextPage: false,
    isError: false,
    fetchNextPage: vi.fn(),
    refetch: vi.fn(),
  }),
  connectPipedreamApp: pipedreamMocks.connect,
}));

beforeAll(() => {
  vi.stubGlobal('scrollTo', vi.fn());
});

beforeEach(() => {
  avatarMocks.upload.mockReset();
  avatarMocks.upload.mockResolvedValue('https://static.example/avatar.png');
  updateSearchParams({ createAgent: undefined });
  setSearchParams.mockClear();
  agentMocks.query.data = [];
  agentMocks.query.isError = false;
  agentMocks.create.mockResolvedValue(undefined);
  agentMocks.delete.mockResolvedValue(undefined);
  agentMocks.update.mockResolvedValue(undefined);
  agentMocks.currentUserId = 'macro|user@example.com';
  agentMocks.currentTeam = { team: { id: 'team-1' } };
  agentMocks.isTeamOwner = false;
  pipedreamMocks.enabled = true;
  pipedreamMocks.connected = [];
  pipedreamMocks.connect.mockReset();
  pipedreamMocks.connect.mockImplementation(
    async (args: { appSlug: string }) => {
      pipedreamMocks.connected = [...pipedreamMocks.connected, args.appSlug];
      return 'connected';
    }
  );
  modelMocks.queries = {
    'in-memory:': successfulModels([
      { id: Model.sonnet55, name: 'Claude Sonnet 4.5' },
      { id: Model.opus55, name: 'Claude Opus 4.5' },
    ]),
  };
});

describe('Agents', () => {
  it('opens the new-agent form from a link and clears the action on cancel', () => {
    updateSearchParams({ createAgent: 'true' });
    render(() => <Agents {...agentsPageProps} />);
    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(
      within(dialog).getByRole('button', { name: 'Create agent' })
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(setSearchParams).toHaveBeenCalledWith(
      { createAgent: undefined },
      { replace: true }
    );
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('lists the built-in global Macro agent as a team agent', () => {
    render(() => <Agents {...agentsPageProps} />);

    const list = screen.getByRole('region', { name: 'Your agents' });

    expect(within(list).getByText('Macro')).toBeTruthy();
    expect(
      within(list)
        .getAllByText(/^@/)
        .map((tag) => tag.textContent)
    ).toEqual(['@macro']);
    expect(within(list).getByText('Team')).toBeTruthy();
    expect(
      within(list).getByText('In Macro · Sonnet 5.5 · All channels')
    ).toBeTruthy();
    expect(
      within(list).queryByRole('button', { name: 'Delete Macro' })
    ).toBeNull();
    expect(
      within(list).queryByRole('button', { name: 'Edit Macro' })
    ).toBeNull();
  });

  it('shows loading and error states in the agent list', () => {
    agentMocks.query.isPending = true;
    agentMocks.query.isSuccess = false;
    try {
      const view = render(() => <Agents {...agentsPageProps} />);
      expect(
        within(screen.getByRole('region', { name: 'Your agents' })).getByText(
          'Loading agents…'
        )
      ).toBeTruthy();
      view.unmount();

      agentMocks.query.isPending = false;
      agentMocks.query.isError = true;
      render(() => <Agents {...agentsPageProps} />);
      expect(
        within(screen.getByRole('region', { name: 'Your agents' })).getByText(
          'Could not load your agents. Try refreshing this page.'
        )
      ).toBeTruthy();
    } finally {
      agentMocks.query.isPending = false;
      agentMocks.query.isSuccess = true;
    }
  });

  it('groups server agents into team and private sections', () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|user@example.com' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          description: 'Finds and fixes bugs.',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
      {
        bot: {
          id: 'agent-2',
          kind: 'owned',
          owner: { type: 'team', team_id: 'team-1' },
          name: 'Release helper',
          handle: 'release-helper',
          description: 'Coordinates releases.',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Keep releases moving.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'selected',
        channel_ids: ['channel-engineering'],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);

    const list = screen.getByRole('region', { name: 'Your agents' });

    // Macro first, then team agents, then private agents.
    expect(
      within(list)
        .getAllByText(/^@/)
        .map((tag) => tag.textContent)
    ).toEqual(['@macro', '@release-helper', '@bug-fixer']);
    expect(within(list).queryByText('Coordinates releases.')).toBeNull();
    expect(within(list).queryByText('Finds and fixes bugs.')).toBeNull();

    const releaseRow = within(list).getByText('Release helper').parentElement
      ?.parentElement as HTMLElement;
    expect(within(releaseRow).getByText('Team')).toBeTruthy();
    expect(
      within(releaseRow).getByText('In Macro · Sonnet 5.5 · #engineering')
    ).toBeTruthy();

    const bugFixerRow = within(list).getByText('Bug fixer').parentElement
      ?.parentElement as HTMLElement;
    expect(within(bugFixerRow).getByText('Private')).toBeTruthy();
    expect(
      within(bugFixerRow).getByText('In Macro · Sonnet 5.5 · All channels')
    ).toBeTruthy();
  });

  it('says where each listed agent runs', () => {
    const owner = { type: 'user', user_id: 'macro|user@example.com' };
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-custom',
          kind: 'owned',
          owner,
          name: 'Custom helper',
          handle: 'custom-helper',
          has_agent: true,
        },
        instructions: '',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    const list = screen.getByRole('region', { name: 'Your agents' });

    expect(
      within(list).getByText('In Macro · Sonnet 5.5 · All channels')
    ).toBeTruthy();
  });

  it('does not list a coworker private agent in Settings', () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-mine',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|user@example.com' },
          name: 'My helper',
          handle: 'my-helper',
          has_agent: true,
        },
        instructions: '',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
      {
        bot: {
          id: 'agent-theirs',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|other@example.com' },
          name: 'Coworker helper',
          handle: 'coworker-helper',
          has_agent: true,
        },
        instructions: '',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    expect(screen.getByText('My helper')).toBeTruthy();
    expect(screen.queryByText('Coworker helper')).toBeNull();
  });

  it("does not list another team's mentionable agent in Settings", () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-other-team',
          kind: 'owned',
          owner: { type: 'team', team_id: 'other-team' },
          name: 'Other Team Helper',
          handle: 'other-team-helper',
          has_agent: true,
        },
        instructions: '',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    expect(screen.queryByText('Other Team Helper')).toBeNull();
  });

  it('uploads a team agent avatar before saving and retains it when reopened', async () => {
    const originalAvatar = 'https://static.example/original.png';
    const uploadedAvatar = 'https://static.example/avatar.png';
    const existing = {
      bot: {
        id: 'agent-1',
        kind: 'owned',
        owner: { type: 'team', team_id: 'team-1' },
        name: 'Grungus',
        handle: 'grungus',
        has_agent: true,
        avatar_url: originalAvatar,
      },
      instructions: 'Help the team.',
      harness: 'in-memory',
      default_model: Model.sonnet55,
      channel_scope: 'all',
      channel_ids: [],
    };
    agentMocks.query.data = [existing];
    let finishUpload!: (url: string) => void;
    avatarMocks.upload.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          finishUpload = resolve;
        })
    );
    agentMocks.update.mockImplementation(
      async (params: { avatarUrl: string }) => {
        existing.bot.avatar_url = params.avatarUrl;
      }
    );
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Grungus' }));
    const form = screen.getByRole('region', { name: 'Edit agent' });
    const file = new File(['avatar'], 'avatar.png', { type: 'image/png' });
    fireEvent.change(form.querySelector('input[type="file"]')!, {
      target: { files: [file] },
    });
    expect(avatarMocks.upload).toHaveBeenCalledWith(file);
    expect(screen.getByRole('button', { name: 'Save changes' })).toHaveProperty(
      'disabled',
      true
    );
    expect(screen.getByRole('button', { name: 'Uploading…' })).toHaveProperty(
      'disabled',
      true
    );
    expect(
      screen.getByRole('img', { name: 'Grungus avatar' }).getAttribute('src')
    ).toBe(originalAvatar);

    finishUpload(uploadedAvatar);
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Save changes' })
      ).toHaveProperty('disabled', false)
    );
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(agentMocks.update).toHaveBeenCalledWith(
        expect.objectContaining({ avatarUrl: uploadedAvatar, teamId: 'team-1' })
      )
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Edit Grungus' })).toBeTruthy()
    );
    fireEvent.click(screen.getByRole('button', { name: 'Edit Grungus' }));
    expect(
      screen.getByRole('img', { name: 'Grungus avatar' }).getAttribute('src')
    ).toBe(uploadedAvatar);
  });

  it('reports avatar upload failures and allows retrying without losing the form', async () => {
    avatarMocks.upload.mockRejectedValueOnce(
      new Error('Failed to upload file')
    );
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));
    fireEvent.input(screen.getByLabelText('Name'), {
      target: { value: 'Grungus' },
    });
    const input = screen
      .getByRole('region', { name: 'New agent' })
      .querySelector('input[type="file"]')!;
    const file = new File(['avatar'], 'avatar.png', { type: 'image/png' });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() =>
      expect(agentMocks.toastFailure).toHaveBeenCalledWith(
        'Failed to upload file'
      )
    );
    expect(screen.getByLabelText('Name')).toHaveProperty('value', 'Grungus');
    expect(screen.getByRole('button', { name: 'Upload' })).toHaveProperty(
      'disabled',
      false
    );
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() =>
      expect(
        screen.getByRole('img', { name: 'Grungus avatar' }).getAttribute('src')
      ).toBe('https://static.example/avatar.png')
    );
  });

  it('edits and persists an existing agent through the agents API', async () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|user@example.com' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          description: 'Finds and fixes bugs.',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'selected',
        channel_ids: ['channel-engineering'],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Bug fixer' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(screen.getByRole('heading', { name: 'Edit agent' })).toBeTruthy();
    expect(within(dialog).getByLabelText('Name')).toHaveProperty(
      'value',
      'Bug fixer'
    );
    expect(within(dialog).getByLabelText('@tag')).toHaveProperty(
      'value',
      'bug-fixer'
    );
    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    expect(within(dialog).getByLabelText('Specific channels')).toHaveProperty(
      'checked',
      true
    );

    fireEvent.input(within(dialog).getByLabelText('Name'), {
      target: { value: 'Bug resolver' },
    });
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Save changes' })
    );

    await waitFor(() => {
      expect(agentMocks.update).toHaveBeenCalledWith({
        agentId: 'agent-1',
        autoAcceptPermissions: true,
        isCoding: false,
        avatarUrl: undefined,
        channelIds: ['channel-engineering'],
        channelScope: 'selected',
        defaultModel: Model.sonnet55,
        description: 'Finds and fixes bugs.',
        handle: 'bug-fixer',
        harness: 'in-memory',
        name: 'Bug resolver',
        instructions: 'Fix the root cause.',
        mcp: { scope: 'owner_connections' },
        teamId: undefined,
      });
      expect(agentMocks.toastSuccess).toHaveBeenCalledWith('Agent updated');
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  it('lets team members edit without deleting or privatizing another creator agent', () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'team', team_id: 'team-1' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          description: 'Finds and fixes bugs.',
          created_by: 'macro|creator@example.com',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);

    expect(screen.getByRole('button', { name: 'Edit Bug fixer' })).toBeTruthy();
    expect(
      screen.queryByRole('button', { name: 'Delete Bug fixer' })
    ).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Edit Bug fixer' }));
    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(within(dialog).getByLabelText('Team')).toHaveProperty(
      'checked',
      true
    );
    expect(within(dialog).getByLabelText('Private')).toHaveProperty(
      'disabled',
      true
    );
    expect(
      within(dialog).getByText('Only the agent creator can make it private.')
    ).toBeTruthy();
  });

  it('lets the creator privatize and the team owner delete a team agent', () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'team', team_id: 'team-1' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          description: 'Finds and fixes bugs.',
          created_by: 'macro|user@example.com',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    const view = render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Bug fixer' }));
    expect(
      within(
        screen.getByRole('region', { name: /^(New|Edit) agent$/ })
      ).getByLabelText('Private')
    ).toHaveProperty('disabled', false);
    view.unmount();

    agentMocks.currentUserId = 'macro|team-owner@example.com';
    agentMocks.isTeamOwner = true;
    render(() => <Agents {...agentsPageProps} />);
    expect(
      screen.getByRole('button', { name: 'Delete Bug fixer' })
    ).toBeTruthy();
  });

  it('confirms before deleting a created agent', async () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|user@example.com' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          description: 'Finds and fixes bugs.',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: Model.sonnet55,
        channel_scope: 'selected',
        channel_ids: ['channel-engineering'],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Delete Bug fixer' }));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('Delete Bug fixer?')).toBeTruthy();
    expect(agentMocks.delete).not.toHaveBeenCalled();

    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Delete agent' })
    );

    await waitFor(() => {
      expect(agentMocks.delete).toHaveBeenCalledWith({
        agentId: 'agent-1',
        channelIds: ['channel-engineering'],
      });
      expect(agentMocks.toastSuccess).toHaveBeenCalledWith('Agent deleted');
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  it('offers agent configuration without description and only connected harnesses', () => {
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(
      screen.getByText('Name it, tell it what to do, and pick where it runs.')
    ).toBeTruthy();

    expect(
      within(dialog).getByRole('button', { name: 'Upload avatar' })
    ).toBeTruthy();
    const avatarInput = dialog.querySelector(
      'input[type="file"]'
    ) as HTMLInputElement;
    fireEvent.change(avatarInput, {
      target: {
        files: [
          new File(['avatar'], 'agent-avatar.png', { type: 'image/png' }),
        ],
      },
    });
    expect(within(dialog).queryByText('agent-avatar.png')).toBeNull();
    expect(
      within(dialog).getByText('Optional · square images work best')
    ).toBeTruthy();
    expect(within(dialog).getByLabelText('Name')).toBeTruthy();
    expect(within(dialog).getByLabelText('@tag')).toBeTruthy();
    expect(within(dialog).queryByLabelText('Description')).toBeNull();
    expect(within(dialog).getByLabelText('Instructions')).toBeTruthy();
    expect(
      within(dialog).getByRole('radio', { name: 'In Macro' })
    ).toHaveProperty('checked', true);
    expect(
      within(dialog).queryByRole('radio', { name: 'In Claude Code' })
    ).toBeNull();
    expect(
      within(dialog).queryByRole('radio', { name: 'In Cursor' })
    ).toBeNull();
    expect(
      within(dialog).queryByRole('radio', { name: 'In your harness' })
    ).toBeNull();
    expect(within(dialog).queryByLabelText('Harness')).toBeNull();
    expect(within(dialog).getAllByLabelText('Default model')).toHaveLength(1);
    expect(within(dialog).getByText('Who can use it')).toBeTruthy();
    expect(within(dialog).getByRole('group', { name: 'Share' })).toBeTruthy();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    expect(
      within(dialog).getByRole('group', { name: 'Channels' })
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByLabelText('Specific channels'));
    const channelSearch =
      within(dialog).getByPlaceholderText('Search channels…');
    fireEvent.input(channelSearch, { target: { value: 'gen' } });
    expect(screen.getByRole('option', { name: 'general' })).toBeTruthy();
  });

  it('keeps channels and connections collapsed under Advanced', () => {
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: 'New agent' });
    const toggle = within(dialog).getByRole('button', { name: 'Show' });
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    expect(
      within(dialog).getByText(
        'Chat agent · All channels · Your connected apps'
      )
    ).toBeTruthy();
    expect(
      within(dialog).queryByRole('group', { name: 'Channels' })
    ).toBeNull();
    expect(
      within(dialog).queryByRole('group', { name: 'Connections' })
    ).toBeNull();

    fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(toggle.textContent).toBe('Hide');
    expect(
      within(dialog).getByRole('group', { name: 'Channels' })
    ).toBeTruthy();
    expect(
      within(dialog).getByRole('group', { name: 'Connections' })
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByLabelText('Specific channels'));
    expect(
      within(dialog).getByText('Chat agent · 0 channels · Your connected apps')
    ).toBeTruthy();

    fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    expect(
      within(dialog).queryByRole('group', { name: 'Channels' })
    ).toBeNull();
  });

  it('explains why team sharing is disabled without a team', () => {
    agentMocks.currentTeam = null;

    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    const teamOption = within(dialog).getByLabelText('Team');
    expect(teamOption).toHaveProperty('disabled', true);
    const teamCardClasses = teamOption.closest('label')?.classList;
    expect(teamCardClasses?.contains('cursor-not-allowed')).toBe(true);
    expect(teamCardClasses?.contains('opacity-50')).toBe(true);
    expect(
      within(dialog).getByText(
        'Team agents need a team owner. Create or join a team in Team settings to enable this option.'
      )
    ).toBeTruthy();
  });

  it('persists creation through the agents API', async () => {
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    fireEvent.input(within(dialog).getByLabelText('Name'), {
      target: { value: 'Bug fixer' },
    });
    fireEvent.input(within(dialog).getByLabelText('Instructions'), {
      target: { value: 'Fix the **root cause** and add tests.' },
    });
    fireEvent.click(within(dialog).getByLabelText('Team'));
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Create agent' })
    );

    await waitFor(() => {
      expect(agentMocks.create).toHaveBeenCalledWith({
        autoAcceptPermissions: true,
        avatarUrl: undefined,
        isCoding: false,
        channelIds: [],
        channelScope: 'all',
        defaultModel: Model.sonnet55,
        handle: 'bug-fixer',
        harness: 'in-memory',
        name: 'Bug fixer',
        instructions: 'Fix the **root cause** and add tests.',
        mcp: { scope: 'owner_connections' },
        teamId: 'team-1',
      });
      expect(agentMocks.toastSuccess).toHaveBeenCalledWith('Agent created');
      expect(screen.queryByRole('dialog')).toBeNull();
    });
  });

  it('preserves and labels a saved model missing from the fresh catalog', () => {
    agentMocks.query.data = [
      {
        bot: {
          id: 'agent-1',
          kind: 'owned',
          owner: { type: 'user', user_id: 'macro|user@example.com' },
          name: 'Bug fixer',
          handle: 'bug-fixer',
          has_agent: true,
          created_at: '2026-08-27T12:00:00Z',
          updated_at: '2026-08-27T12:00:00Z',
        },
        instructions: 'Fix the root cause.',
        harness: 'in-memory',
        default_model: 'retired-model',
        channel_scope: 'all',
        channel_ids: [],
      },
    ];

    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Bug fixer' }));

    const defaultModel = within(
      screen.getByRole('region', { name: /^(New|Edit) agent$/ })
    ).getByLabelText('Default model');
    expect(defaultModel).toHaveProperty(
      'textContent',
      'retired-model (saved, unavailable)'
    );
    expect(
      selectOptions(defaultModel).getByRole('option', {
        name: 'retired-model (saved, unavailable)',
      })
    ).toBeTruthy();
  });

  const linearAgent = {
    bot: {
      id: 'agent-mcp',
      kind: 'owned',
      owner: { type: 'team', team_id: 'team-1' },
      name: 'Triage bot',
      handle: 'triage-bot',
      has_agent: true,
      created_by: 'macro|user@example.com',
      created_at: '2026-08-27T12:00:00Z',
      updated_at: '2026-08-27T12:00:00Z',
    },
    instructions: '',
    harness: 'in-memory',
    default_model: Model.sonnet55,
    channel_scope: 'all',
    channel_ids: [],
    mcp: {
      scope: 'selected',
      servers: [
        { app_slug: 'linear', server_name: 'Linear' },
        { app_slug: 'notion', server_name: 'Notion' },
      ],
    },
  };

  it('hides the Connections section when the Pipedream stack is off but keeps the selection', async () => {
    pipedreamMocks.enabled = false;
    agentMocks.query.data = [linearAgent];
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Triage bot' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    expect(
      within(dialog).getByRole('group', { name: 'Channels' })
    ).toBeTruthy();
    expect(within(dialog).queryByText('Connections')).toBeNull();
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Save changes' })
    );

    await waitFor(() => {
      expect(agentMocks.update).toHaveBeenCalledWith(
        expect.objectContaining({
          mcp: { scope: 'selected', servers: linearAgent.mcp.servers },
        })
      );
    });
  });

  it('defaults new agents to the owner connections policy', () => {
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(
      within(dialog).getByText(
        'Chat agent · All channels · Your connected apps'
      )
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    expect(
      within(dialog).getByLabelText('Use my connected apps')
    ).toHaveProperty('checked', true);
    expect(within(dialog).queryByLabelText('Search connectors')).toBeNull();
  });

  it('picks apps from the whole catalog, shows the viewer connection state, and persists them', async () => {
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add agent' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    fireEvent.input(within(dialog).getByLabelText('Name'), {
      target: { value: 'Triage bot' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    fireEvent.click(within(dialog).getByLabelText('Specific apps'));

    // An explicit selection with nothing in it is not something to save.
    expect(
      within(dialog).getByRole('button', { name: 'Create agent' })
    ).toHaveProperty('disabled', true);

    within(dialog).getByLabelText('Search connectors').focus();
    fireEvent.input(within(dialog).getByLabelText('Search connectors'), {
      target: { value: 'lin' },
    });
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /Linear/ })).toBeTruthy();
    });
    fireEvent.click(screen.getByRole('option', { name: /Linear/ }));

    // The pick is listed as not connected for this viewer, with a way in.
    await waitFor(() => {
      expect(within(dialog).getByText('Linear')).toBeTruthy();
    });
    expect(within(dialog).getByLabelText('Not connected')).toBeTruthy();
    expect(
      within(dialog).getByRole('button', { name: 'Connect' })
    ).toBeTruthy();
    // ...and no longer offered by the search.
    fireEvent.input(within(dialog).getByLabelText('Search connectors'), {
      target: { value: 'lin' },
    });
    await waitFor(() => {
      expect(screen.queryByRole('option', { name: /Linear/ })).toBeNull();
    });

    // Unconnected picks never block saving: the agent's author chooses the
    // apps, each person connects their own.
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Create agent' })
    );
    await waitFor(() => {
      expect(agentMocks.create).toHaveBeenCalledWith(
        expect.objectContaining({
          mcp: {
            scope: 'selected',
            servers: [{ app_slug: 'linear', server_name: 'Linear' }],
          },
        })
      );
    });
  });

  it('connects a picked app in place and flips its indicator without saving', async () => {
    agentMocks.update.mockClear();
    agentMocks.query.data = [linearAgent];
    render(() => <Agents {...agentsPageProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Triage bot' }));

    const dialog = screen.getByRole('region', { name: /^(New|Edit) agent$/ });
    expect(
      within(dialog).getByText('Chat agent · All channels · 2 selected apps')
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Show' }));
    expect(within(dialog).getByLabelText('Specific apps')).toHaveProperty(
      'checked',
      true
    );
    expect(within(dialog).getAllByLabelText('Not connected')).toHaveLength(2);
    // Team agents say out loud that connections are per person.
    expect(within(dialog).getByText(/Connections are personal/)).toBeTruthy();

    // Linear is listed first; its row carries the first Connect action.
    fireEvent.click(
      within(dialog).getAllByRole('button', {
        name: 'Connect',
      })[0] as HTMLElement
    );

    await waitFor(() => {
      expect(pipedreamMocks.connect).toHaveBeenCalledWith(
        expect.objectContaining({ appSlug: 'linear', serverName: 'Linear' })
      );
    });
    expect(agentMocks.update).not.toHaveBeenCalled();

    // Removing one pick keeps the other.
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Remove Notion' })
    );
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Save changes' })
    );
    await waitFor(() => {
      expect(agentMocks.update).toHaveBeenCalledWith(
        expect.objectContaining({
          mcp: {
            scope: 'selected',
            servers: [{ app_slug: 'linear', server_name: 'Linear' }],
          },
        })
      );
    });
  });
});
