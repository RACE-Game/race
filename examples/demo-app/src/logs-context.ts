import { Event } from 'race-sdk';
import React from 'react';

type LogsData = {
  addLog: (e: Event) => void,
  clearLog: () => void,
};

const LogsContext =
  React.createContext<LogsData>({
    addLog: (_: Event) => { },
    clearLog: () => {},
  });

export default LogsContext;
