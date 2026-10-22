/** Session kind used for routes and the agent workspace's presentation. */
export type AgentsMode = 'chat';

/** The composer's mode switch calls chat agents "Chat". */
export function agentsModeLabel(_mode?: AgentsMode): string {
  return 'Chat';
}
