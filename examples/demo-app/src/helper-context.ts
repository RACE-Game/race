import React, { Context } from 'react';
import { AppHelper } from 'race-sdk';

const HelperContext: Context<AppHelper | undefined> =
  React.createContext(undefined);

export default HelperContext;
