/**
 * @vitest-environment jsdom
 */

import { agentHarnessServiceClient } from '@service-agent-harness/client';
import { QueryClient, QueryClientProvider } from '@tanstack/solid-query';
import { ok } from 'neverthrow';
import type { JSX } from 'solid-js';
import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { buildAgentModelTargets, useAgentModelsQueries } from './models';

vi.mock('@service-agent-harness/client', () => ({
  agentHarnessServiceClient: {
    loadAgentModels: vi.fn(),
  },
}));

let queryClient: QueryClient;
let dispose: (() => void) | undefined;

function renderHook(factory: () => unknown) {
  dispose = render(
    () => (
      <QueryClientProvider client={queryClient}>
        {(() => {
          factory();
          return null as unknown as JSX.Element;
        })()}
      </QueryClientProvider>
    ),
    document.body
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
});

afterEach(() => {
  dispose?.();
  dispose = undefined;
  queryClient.clear();
});

describe('agent model discovery', () => {
  it('constructs the in-memory target', async () => {
    const targets = buildAgentModelTargets();
    const pending = new Promise<never>(() => {});
    vi.mocked(agentHarnessServiceClient.loadAgentModels).mockReturnValue(
      pending
    );

    renderHook(() => useAgentModelsQueries(() => targets));

    await vi.waitFor(() => {
      expect(agentHarnessServiceClient.loadAgentModels).toHaveBeenCalledTimes(
        1
      );
    });
    expect(
      vi
        .mocked(agentHarnessServiceClient.loadAgentModels)
        .mock.calls.map(([request]) => request)
    ).toEqual([{ harness: 'in-memory' }]);
  });

  it('keeps a loaded catalog across remounts', async () => {
    vi.mocked(agentHarnessServiceClient.loadAgentModels).mockResolvedValue(
      ok({
        status: 'available',
        models: [],
      })
    );

    renderHook(() =>
      useAgentModelsQueries(() => [{ harness: 'in-memory' }])
    );
    await vi.waitFor(() => {
      expect(agentHarnessServiceClient.loadAgentModels).toHaveBeenCalledTimes(
        1
      );
      expect(
        queryClient
          .getQueryCache()
          .getAll()
          .every((query) => query.state.status === 'success')
      ).toBe(true);
    });

    dispose?.();
    renderHook(() =>
      useAgentModelsQueries(() => [{ harness: 'in-memory' }])
    );
    await vi.waitFor(() => {
      expect(
        queryClient
          .getQueryCache()
          .getAll()
          .every((query) => query.state.status === 'success')
      ).toBe(true);
    });
    expect(agentHarnessServiceClient.loadAgentModels).toHaveBeenCalledTimes(1);
  });
});
