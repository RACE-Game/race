import { AppClient } from 'race-sdk';
import React from 'react';

type ContextData = {
  context: any,
  state: any,
  account: any,
  client: AppClient | undefined,
}

const GameContext =
  React.createContext<ContextData>({
    context: undefined,
    state: undefined,
    account: undefined,
    client: undefined,
  });

export default GameContext;
