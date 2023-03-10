import { useContext, useEffect, useRef, useState } from "react";
import { useParams } from 'react-router-dom';
import { AppClient, Event } from 'race-sdk';
import { CHAIN, RPC } from "./constants";
import ProfileContext from "./profile-context";
import LogsContext from "./logs-context";
import Header from "./Header";

type GameStage = "Dealing" | "Betting" | "Reacting" | "Revealing";

interface Player {
  addr: string,
  balance: bigint,
  bet: bigint,
}

interface State {
  last_winner: string | null,
  random_id: bigint,
  players: Player[],
  stage: GameStage,
  bet: bigint,
  blind_bet: bigint,
  min_bet: bigint,
  max_bet: bigint,
}

function DrawCard() {
  let [state, setState] = useState<State | undefined>(undefined);
  let [context, setContext] = useState<any | undefined>(undefined);
  let client = useRef<AppClient | undefined>(undefined);
  let { addr } = useParams();
  let profile = useContext(ProfileContext);
  let { addLog, clearLog } = useContext(LogsContext);

  const onEvent = (context: any, state: State, event: Event | undefined) => {
    if (event !== undefined) {
      addLog(event);
    }
    setContext(context);
    setState(state);
  }

  useEffect(() => {
    const initClient = async () => {
      if (profile !== undefined && addr !== undefined) {
        console.log("App client created");
        let c = await AppClient.try_init(CHAIN, RPC, profile.addr, addr, onEvent);
        client.current = c;
        await c.attach_game();
        console.log("Attached to game");
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

  console.log(addr, state);

  if (addr === undefined || state === undefined) return null;

  return (
    <div className="h-full w-full flex flex-col">
      <Header gameAddr={addr} />
      DrawCard
    </div>
  );
}

export default DrawCard;
