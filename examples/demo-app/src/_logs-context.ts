import { GameEvent } from '@race/sdk-core';
import React from 'react';

export type LogsData = {
  addLog: (e: Event) => void,
  clearLog: () => void,
};

export const LogsContext =
  React.createContext<LogsData>({
    addLog: (_: Event) => { },
    clearLog: () => {},
  });
