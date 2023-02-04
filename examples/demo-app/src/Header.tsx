import { useContext } from "react";
import GameContext from "./game-context";

function Header() {
  const { context, client } = useContext(GameContext);

  const onJoin = () => {
    if (client !== undefined) {
      client.join(0, 0n);
    }
  }

  if (context === undefined) {
    return <div></div>;
  } else {
    return (
      <div className="w-full h-full p-2 flex">
        <div className="flex-1 flex flex-wrap">
          <div className="m-2"> <span className="font-bold">Address:</span> {context.game_addr}</div>
          <div className="m-2"> <span className="font-bold">Status:</span> {context.status}</div>
          <div className="m-2"> <span className="font-bold">Servers:</span> {context.servers.length}</div>
          <div className="m-2"> <span className="font-bold">Clients:</span> {context.players.length} ({context.pending_players.length})</div>
          <div className="m-2"> <span className="font-bold">Settles:</span> {context.settle_version}</div>
          <div className="m-2"> <span className="font-bold">Accesses:</span> {context.access_version}</div>
        </div>
        <button className="rounded-full border border-black hover:bg-gray-200 active:scale-[90%]
transition-all w-20 h-20 self-center"
          onClick={onJoin}>
          + Join
        </button>
      </div>
    )
  }
}

export default Header;
