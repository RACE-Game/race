import { useContext, useEffect, useRef, useState } from "react";
import { useParams } from 'react-router-dom';
import { AppClient, Event } from 'race-sdk';
import { CHAIN, RPC } from "./constants";
import ProfileContext from "./profile-context";
import LogsContext from "./logs-context";
import Header from "./Header";

interface Player {
  addr: string,
  balance: bigint,
}

interface State {
  last_winner: string | null,
  players: Player[],
  random_id: number,
  draw_time: bigint,
}

function Winner(props: { settle_version: number, last_winner: string | null }) {

  const [fade, setFade] = useState(false);

  useEffect(() => {
    setFade(false);
    setTimeout(() => setFade(true), 5000)
  }, [props.settle_version]);

  if (props.last_winner) {
    return <div className={
      `bg-black text-white text-lg p-4 text-center animate-bounce transition-opacity duration-[3500ms]
       ${fade ? "opacity-0" : "opacity-100"}`}>
      Winner: {props.last_winner}
    </div>
  } else {
    return <div></div>
  }
}

function Raffle() {
  let [state, setState] = useState<State | undefined>(undefined);
  let [context, setContext] = useState<any | undefined>(undefined);
  let client = useRef<AppClient | undefined>(undefined);
  let { addr } = useParams();
  let profile = useContext(ProfileContext);
  let { addLog, clearLog } = useContext(LogsContext);

  // Game event handler
  const onEvent = (context: any, state: State, event: Event | undefined) => {
    console.log(event?.kind(), event?.data(), state);
    if (event !== undefined) {
      addLog(event);
    }
    setContext(context);
    setState(state);
  }

  // Button callback to join the raffle
  const onJoin = async () => {
    if (client.current !== undefined) {
      await client.current.join(0, 100n);
    }
  }

  // Initialize app client
  useEffect(() => {
    const initClient = async () => {
      if (profile !== undefined && addr !== undefined) {
        console.log("Create AppClient");
        let c = await AppClient.try_init(CHAIN, RPC, profile.addr, addr, onEvent);
        client.current = c;
        await c.attach_game();
      }
    };
    initClient();
    return () => {
      clearLog();
      if (client.current) {
        client.current.close();
      }
    }
  }, [profile, addr]);

  if (addr == undefined || state === undefined || context === undefined) {
    return <svg className="animate-spin h-5 w-5 mr-3" viewBox="0 0 24 24"></svg>
  } else {
    return (
      <div className="h-full w-full flex flex-col">
        <Header gameAddr={addr} />
        <div className="font-bold m-4 flex">
          <div>Raffle @ {addr}</div>
          <div className="flex-1"></div>
          <button
            onClick={onJoin}
            className="px-4 py-1 bg-black text-white rounded-md">Join</button>
        </div>
        <div>
          Next draw: {
            state.draw_time > 0 ? new Date(Number(state.draw_time)).toLocaleTimeString() : "N/A"
          }
        </div>
        <div>Players:</div>
        {
          state.players.map((p, i) => <div key={i} className="m-2 p-2 border border-black">
            {p.addr}
          </div>)
        }

        <div className="flex-1"></div>

        <Winner
          last_winner={state.last_winner}
          settle_version={context.settle_version} />
      </div>
    );
  }
}


export default Raffle;
