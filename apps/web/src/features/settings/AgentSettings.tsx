import { Agents } from './Agents';

export type AgentManagementSection = 'agents';

/** One management surface, mounted by both Settings and the Agents workspace. */
export function AgentSettings(_props?: {
  initialSection?: AgentManagementSection;
}) {
  return <Agents />;
}
