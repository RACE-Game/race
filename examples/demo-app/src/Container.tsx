import { AppClient } from 'race-sdk';
import { useContext, useEffect, useState } from 'react';
import Chat from './Chat';
import Raffle from './Raffle';
import { CHAIN, RPC } from './constants';
import GameContext from './game-context';
import HelperContext from './helper-context';
import ProfileContext from './profile-context';

type ContainerProps = {
  gameAddr: string | undefined
}

function Example(props: { account: any, profile: any }) {
  const { account, profile } = props;
  const [state, setState] = useState(undefined);
  const { client, setClient, setContext } = useContext(GameContext);

  const onEvent = (_addr: string, context: any, state: any) => {
    setState(state);
    setContext(context);
  }

  useEffect(() => {
    if (account !== undefined && profile !== undefined) {
      console.log("Connect to game: ", account.addr);
      AppClient.try_init(CHAIN, RPC, profile.addr, account.addr, onEvent).then(client => {
        setClient(client);
        client.attach_game();
      });
    }
  }, [profile, account.addr]);


  if (state === undefined) {
    return <h1>Initializing state</h1>
  } else if (account.bundle_addr === "CHAT_BUNDLE_ADDRESS") {
    return <Chat account={account} profile={profile} state={state} />;
  } else if (account.bundle_addr === "RAFFLE_BUNDLE_ADDRESS") {
    return <Raffle account={account} profile={profile} state={state} />;
  } else {
    return <div></div>;
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
  } else if (account === undefined) {
    return (
      <div className="w-full h-full flex flex-col justify-center items-center text-2xl text-gray-500">
        <p>Loading...</p>
      </div>
    )
  } else {
    return (
      <div className="w-full h-full overflow-hidden">
        <Example key={props.gameAddr} account={account} profile={profile} />
      </div>
    )
  }
}

export default Container;
