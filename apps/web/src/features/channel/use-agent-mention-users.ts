import type { IUser } from '@core/user/types';
import { uniqueByKey } from '@core/util/compareUtils';
import type { Accessor } from 'solid-js';
import { isMacroAiId, isMacroNewId, macroMentionUser } from './macroAi';
import { useChatV3AgentsFlag } from './use-chat-v3-agents-flag';

/**
 * Built-in agent entries shared by every message composer.
 * Macro gets exactly one entry. Both Macro bots are dropped from the incoming
 * list first, so a stray participant or channel-bot row cannot put a second
 * "Macro" in the menu; the rollout then decides which id that one entry carries.
 */
export function useAgentMentionUsers(
  users: Accessor<IUser[]>,
  enabled: Accessor<boolean> = () => true
): Accessor<IUser[]> {
  const canUseAgents = useChatV3AgentsFlag();

  return () => {
    const mentionable = users();
    if (!enabled()) return mentionable;
    const base = mentionable.filter(
      (user) => !isMacroAiId(user.id) && !isMacroNewId(user.id)
    );
    base.unshift(macroMentionUser(canUseAgents()));
    return uniqueByKey(base, (user) => user.id);
  };
}
