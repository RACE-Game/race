import { Event } from 'race-sdk';
import React from 'react';

type LogsData = {
  addLog: (e: Event) => void,
};

const LogsContext =
  React.createContext<LogsData>({
    addLog: (_: Event) => { },
  });

export default LogsContext;
