import * as React from 'react';
import { FacadeWallet } from '@race/sdk-facade';

export const DEFAULT_WALLET = new FacadeWallet();

export const FacadeWalletContext =
  React.createContext<FacadeWallet>(DEFAULT_WALLET)
