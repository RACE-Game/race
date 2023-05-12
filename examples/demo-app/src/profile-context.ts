import { PlayerProfile } from '@race/sdk-core';
import React from 'react';

export const ProfileContext =
  React.createContext<PlayerProfile | undefined>(undefined);
