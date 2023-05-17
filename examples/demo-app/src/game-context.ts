import { AppClient } from '@race/sdk-core';
import React from 'react';

export type GameContextData = {
  context: any,
  setContext: (context: any) => void,
  client: AppClient | undefined,
  setClient: (client: AppClient) => void,
}

const GameContext =
  React.createContext<GameContextData>({
    context: undefined,
    setContext: (_: any) => { },
    client: undefined,
    setClient: (_: AppClient) => { },
  });

export default GameContext;
