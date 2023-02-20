import React from 'react';
import { AppHelper } from 'race-sdk';

const HelperContext =
  React.createContext<AppHelper | undefined>(undefined);

export default HelperContext;
