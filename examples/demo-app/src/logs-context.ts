import { Event } from 'race-sdk';
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
