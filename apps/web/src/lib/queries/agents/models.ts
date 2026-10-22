import { throwOnErr } from '@core/util/result';
import { agentHarnessServiceClient } from '@service-agent-harness/client';
import type {
  LoadAgentModelsRequest,
  LoadAgentModelsResponse,
} from '@service-agent-harness/generated/schemas';
import { useQueries, useQuery } from '@tanstack/solid-query';

export type AgentModelTarget = LoadAgentModelsRequest;

/** Every model provider available to the agent dialog, in display order. */
export function buildAgentModelTargets(): AgentModelTarget[] {
  return [{ harness: 'in-memory' }];
}

/** The in-memory catalog depends on current plan permissions. */
const IN_MEMORY_MODELS_STALE_TIME = 60 * 1000;

const MODELS_GC_TIME = 30 * 60 * 1000;

export function agentModelsQueryOptions(target: AgentModelTarget) {
  return {
    queryKey: [
      'agent-models',
      'load',
      target.harness,
      target.harnessId ?? null,
    ] as const,
    queryFn: async ({
      signal,
    }: {
      signal: AbortSignal;
    }): Promise<LoadAgentModelsResponse> =>
      await throwOnErr(() =>
        agentHarnessServiceClient.loadAgentModels(target, signal)
      ),
    gcTime: MODELS_GC_TIME,
    staleTime: IN_MEMORY_MODELS_STALE_TIME,
    retry: false,
  };
}

/** Launches one independent, fresh model-discovery request per target. */
export function useAgentModelsQueries(
  targets: () => readonly AgentModelTarget[]
) {
  return useQueries(() => ({
    queries: targets().map(agentModelsQueryOptions),
  }));
}

/** Loads one provider's current model catalog. */
export function useAgentModelsQuery(
  target: () => AgentModelTarget,
  enabled: () => boolean = () => true
) {
  return useQuery(() => ({
    ...agentModelsQueryOptions(target()),
    enabled: enabled(),
  }));
}
