import { GameEvent } from '@race-foundation/sdk-core';
import React from 'react';

export type LogsData = {
  addLog: (e: GameEvent) => void,
  clearLog: () => void,
};

export const LogsContext =
  React.createContext<LogsData>({
    addLog: (_: GameEvent) => { },
    clearLog: () => { },
  });
