import React from 'react';

export interface ProfileData {
  addr: string,
  nick: string,
  pfp: string,
}

const ProfileContext =
  React.createContext<ProfileData | undefined>(undefined);

export default ProfileContext;
