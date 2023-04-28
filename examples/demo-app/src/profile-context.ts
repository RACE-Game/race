import React from 'react';

export interface ProfileData {
  addr: string,
  nick: string,
  pfp: string,
}

export const ProfileContext =
  React.createContext<ProfileData | undefined>(undefined);
