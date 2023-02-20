import React, { useContext, useState } from 'react';
import HelperContext from './helper-context';
import ProfileContext from './profile-context';

function Profile() {
  let [nick, setNick] = useState<string>("");
  let helper = useContext(HelperContext);
  let { profile, setProfile } = useContext(ProfileContext);

  const editNick = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNick(e.target.value);
  }

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      setNick(e.currentTarget.value);
    }
  }

  const createProfile = async () => {
    if (helper !== undefined) {
      if (nick === "") {
        alert("Can't be empty");
      } else {
        await helper.create_profile(nick, nick, "");
        const profile = await helper.get_profile(nick);
        console.log(profile);
        setProfile(profile);
      }
    }
  }

  return (
    <div className="grid place-items-center border border-gray-500 rounded-lg">
      <input
        className="text-gray-800 focus:text-black bg-transparent border-b border-black outline-none text-center p-4 text-lg"
        name="nick"
        type="text"
        placeholder="Enter nick"
        disabled={helper === undefined || profile !== undefined}
        onChange={editNick}
        onKeyDown={onKeyDown}
        value={nick} />

      {
        profile !== undefined ?
          <div className="text-gray-500">
            Connected
          </div> :
          <button className="px-4 py-2 rounded-lg border border-black hover:bg-gray-100 active:bg-gray-200 transition-all active:translate-y-1"
            onClick={createProfile}>
            Create Profile
          </button>
      }
    </div >
  )
}

export default Profile;