import React from 'react';
import { AppHelper } from '@race-foundation/sdk-core';

export const HelperContext =
  React.createContext<AppHelper | undefined>(undefined);
