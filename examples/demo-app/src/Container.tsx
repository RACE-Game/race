import { AppClient } from 'race-sdk';
import { useContext, useEffect, useState } from 'react';
import Chat from './Chat';
import { CHAIN, RPC } from './constants';
import HelperContext from './helper-context';
import ProfileContext from './profile-context';

type ContainerProps = {
  gameAddr: string | undefined
}

function renderExample(account: any, profile: any) {
  if (account === undefined) {
    <h1>Fetching account</h1>
  } else if (account.bundle_addr === "CHAT_BUNDLE_ADDRESS") {
    return <Chat account={account} profile={profile} />;
  } else {
    return null;
  }
}

function Container(props: ContainerProps) {
  const [account, setAccount] = useState<any | undefined>(undefined);
  const { profile } = useContext(ProfileContext);
  const helper = useContext(HelperContext);
  useEffect(() => {
    if (props.gameAddr !== undefined && helper !== undefined) {
      helper.get_game_account(props.gameAddr).then(setAccount);
    }
  }, [props.gameAddr]);

  if (helper === undefined) {
    return (
      <div className="w-full h-full flex flex-col justify-center items-center text-2xl text-gray-500">
        <p>Initializing ...</p>
      </div>
    )
  } else if (profile === undefined) {
    return (
      <div className="w-full h-full flex flex-col justify-center items-center text-2xl text-gray-500">
        <p>Profile required</p>
      </div>
    )
  } else if (props.gameAddr === undefined) {
    return (
      <div className="w-full h-full flex flex-col justify-center items-center text-2xl text-gray-500">
        <p>Race Protocol Demo</p>
        <p>Select one from left</p>
      </div>
    );
  } else {
    return (
      <div className="w-full h-full">
        {renderExample(account, profile)}
      </div>
    )
  }
}

export default Container;
