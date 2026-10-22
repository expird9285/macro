import { createQueryKeys } from '@lukemorales/query-key-factory';

export const agentSessionKeys = createQueryKeys('agentSessionMentions', {
  detail: (id: string) => ({ queryKey: [id] }),
  preview: (id: string, graphql: boolean) => ({ queryKey: [id, graphql] }),
});

/** Pull request links, looked up from either side. */
export const agentSessionPullRequestKeys = createQueryKeys(
  'agentSessionPullRequests',
  {
    forPullRequest: (url: string) => ({ queryKey: [url] }),
    forSession: (sessionId: string) => ({ queryKey: [sessionId] }),
    /** Everything a batch of pull requests links to, keyed by their URLs. */
    linksForPullRequests: (urls: readonly string[]) => ({
      queryKey: [urls],
    }),
  }
);

/** Speculative, hidden sessions prepared for the signed-in owner. */
export const agentSessionWarmKeys = createQueryKeys('agentSessionWarm', {
  owner: (userId: string | undefined) => ({ queryKey: [userId] }),
});

/** Durable code-mode records, scoped to their owning session. */
export const agentCodeExecutionKeys = createQueryKeys('agentCodeExecution', {
  detail: (sessionId: string, executionId: string) => ({
    queryKey: [sessionId, executionId],
  }),
});
