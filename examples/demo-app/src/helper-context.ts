import React from 'react';
import { AppHelper } from 'race-sdk';

export const HelperContext =
  React.createContext<AppHelper | undefined>(undefined);
