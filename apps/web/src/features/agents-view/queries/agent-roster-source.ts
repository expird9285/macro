import { useAgentsQuery } from '@queries/agents/agents';
import { useAgentModelsQuery } from '@queries/agents/models';
import { type Accessor, createMemo } from 'solid-js';
import { buildAgentRoster, type RosterAgent } from '../core/roster';

export type AgentRosterSource = {
  /** Every agent the caller can see, first-party ones first. */
  roster: Accessor<RosterAgent[]>;
  /** The saved agents are still on their first load. */
  loading: Accessor<boolean>;
  /** Availability loading state. Always false as runtimes are internal. */
  availabilityLoading: Accessor<boolean>;
  /** The saved agents failed to load; the first-party ones are still listed. */
  error: Accessor<boolean>;
  cursorConnected: Accessor<boolean>;
};

/**
 * Joins the saved agents into one roster.
 */
export function createAgentRosterSource(): AgentRosterSource {
  const agentsQuery = useAgentsQuery();
  const macroDefaults = useAgentModelsQuery(() => ({ harness: 'in-memory' }));

  const agents = () => (agentsQuery.isSuccess ? agentsQuery.data : []);
  const roster = createMemo(() =>
    buildAgentRoster({
      agents: agents(),
      macroDefaultModel: macroDefaults.isSuccess
        ? (macroDefaults.data.currentModel ?? undefined)
        : undefined,
    })
  );

  return {
    roster,
    loading: () => agentsQuery.isPending,
    availabilityLoading: () => false,
    error: () => agentsQuery.isError,
    cursorConnected: () => false,
  };
}
