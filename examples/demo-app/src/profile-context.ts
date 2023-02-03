import React, { Context } from 'react';

export interface Profile {
  addr: string,
  nick: string,
  pfp: string,
}

export interface ProfileData {
  profile: Profile | undefined,
  setProfile: (p: Profile) => void,
}

const ProfileContext =
  React.createContext<ProfileData>({
    profile: undefined,
    setProfile: (_: Profile) => { },
  });

export default ProfileContext;
