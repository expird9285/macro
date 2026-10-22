import type {
  CodingPreferencesBody,
  SandboxSize,
} from '../../../generated/agent-harness/types.gen';
import type { MacroClient } from '../../utils/client';
import {
  AgentSession,
  type CreateManagedSessionOptions,
} from './agent-session';

/** Entry point for agent sessions. */
export class AgentSessionNamespace {
  constructor(private readonly client: MacroClient) {}

  /** A handle to an agent session by id. Details load on first access. */
  byId(id: string): AgentSession {
    return AgentSession.byId(this.client, id);
  }

  /** Create a managed agent session. */
  createManaged(opts?: CreateManagedSessionOptions): Promise<AgentSession> {
    return AgentSession.createManaged(this.client, opts);
  }

  /**
   * The sessions linked to the GitHub pull request at `url` that the caller
   * can view.
   */
  forPullRequest(url: string): Promise<AgentSession[]> {
    return AgentSession.forPullRequest(this.client, url);
  }

  /** The caller's default sandbox size for new `@coder` sessions. */
  defaultSandboxSize(): Promise<SandboxSize> {
    return AgentSession.defaultSandboxSize(this.client);
  }

  /** Set the caller's default sandbox size for the next `@coder` mention. */
  setDefaultSandboxSize(size: SandboxSize): Promise<SandboxSize> {
    return AgentSession.setDefaultSandboxSize(this.client, size);
  }

  /** What the caller's new coding sessions are told to do beyond their assignment. */
  codingPreferences(): Promise<CodingPreferencesBody> {
    return AgentSession.codingPreferences(this.client);
  }

  /** Replace the caller's coding preferences. */
  setCodingPreferences(
    preferences: CodingPreferencesBody,
  ): Promise<CodingPreferencesBody> {
    return AgentSession.setCodingPreferences(this.client, preferences);
  }
}
