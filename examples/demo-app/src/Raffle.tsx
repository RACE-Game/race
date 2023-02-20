import { useContext, useEffect, useState } from "react";
import { useParams } from 'react-router-dom';
import { AppClient, Event } from 'race-sdk';
import { CHAIN, RPC } from "./constants";
import ProfileContext from "./profile-context";
import LogsContext from "./logs-context";

interface State {
  random_id: number,
  options: string[],
  previous_winner: string | null,
  next_draw: number,
}


function Winner(props: { settle_version: number, previous_winner: string | null }) {

  const [fade, setFade] = useState(false);

  useEffect(() => {
    setFade(false);
    setTimeout(() => setFade(true), 5000)
  }, [props.settle_version]);

  if (props.previous_winner) {
    return <div className={
      `bg-black text-white text-lg p-4 text-center animate-bounce transition-opacity duration-[3500ms]
       ${fade ? "opacity-0" : "opacity-100"}`}>
      Winner: {props.previous_winner}
    </div>
  } else {
    return <div></div>
  }
}

function Raffle() {
  let [state, setState] = useState<State | undefined>(undefined);
  let [context, setContext] = useState<any | undefined>(undefined);
  let [client, setClient] = useState<AppClient | undefined>(undefined);
  let { addr } = useParams();
  let profile = useContext(ProfileContext);
  let { addLog } = useContext(LogsContext);

  // Game event handler
  const onEvent = (context: any, state: State, event: Event | null) => {
    if (event !== null) {
      addLog(event);
    }
    setContext(context);
    setState(state);
  }

  // Button callback to join the raffle
  const onJoin = async () => {
    if (client !== undefined) {
      await client.join(0, 100n);
    }
  }

  // Initialize app client
  useEffect(() => {
    const initClient = async () => {
      if (profile !== undefined && addr !== undefined) {
        console.log("Create AppClient");
        let client = await AppClient.try_init(CHAIN, RPC, profile.addr, addr, onEvent);
        setClient(client);
        await client.attach_game();
      }
    };
    initClient();
  }, [profile, addr]);

  if (state === undefined || context === undefined) {
    return <svg className="animate-spin h-5 w-5 mr-3" viewBox="0 0 24 24"></svg>
  } else {
    return (
      <div className="h-full w-full flex flex-col">
        <div className="font-bold m-4 flex">
          <div>Raffle @ {addr}</div>
          <div className="flex-1"></div>
          <button
            onClick={onJoin}
            className="px-4 py-1 bg-black text-white rounded-md">Join</button>
        </div>
        <div>
          Next draw: {
            state.next_draw > 0 ? new Date(state.next_draw).toLocaleTimeString() : "N/A"
          }
        </div>
        <div>Players:</div>
        {
          context.pending_players.map((p: any, i: number) => {
            return <div key={i} className="m-2 p-2 border border-black">
              {p.addr}
            </div>
          })
        }

        <div className="flex-1"></div>
        <Winner
          previous_winner={state.previous_winner}
          settle_version={context.settle_version} />
      </div>
    );
  }
}

export default Raffle;
