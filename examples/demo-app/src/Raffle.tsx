import { useContext, useEffect, useState } from "react";
import { AppClient } from 'race-sdk';
import GameContext from "./game-context";

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
    setTimeout(() => setFade(true), 1000)
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

function Raffle(props: { profile: any, account: any, state: State }) {
  const { profile, account, state } = props;
  const { context } = useContext(GameContext);

  if (state.options === undefined) return null;

  return (
    <div className="w-full h-full p-4 flex flex-col">
      <div className="font-bold m-4">
        Next draw: {state.next_draw > 0 ? new Date(state.next_draw).toLocaleTimeString() : "N/A"}
      </div>
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

export default Raffle;
