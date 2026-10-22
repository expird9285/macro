import { type Accessor, createContext, useContext } from 'solid-js';
import type { AgentsMode } from '../core/mode';

/**
 * The design's `data-mode` vocabulary: `macro` is Chat.
 */
export type DataMode = 'macro';

export function dataModeFor(_mode?: AgentsMode): DataMode {
  return 'macro';
}

const defaultMode: Accessor<DataMode> = () => 'macro';
const DataModeContext = createContext(defaultMode);

export const DataModeProvider = DataModeContext.Provider;

export function useDataMode(): Accessor<DataMode> {
  return useContext(DataModeContext);
}
