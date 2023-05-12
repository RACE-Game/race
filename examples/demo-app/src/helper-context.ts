import React from 'react';
import { AppHelper } from '@race/sdk-core';

export const HelperContext =
  React.createContext<AppHelper | undefined>(undefined);
