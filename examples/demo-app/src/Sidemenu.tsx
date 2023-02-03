import { AppHelper } from 'race-sdk';
import React, { useContext, useState, useEffect } from 'react';
import HelperContext from './helper-context';

interface GameRegistration {
  addr: string,
  bundle_addr: string,
  title: string,
  reg_time: number
}

type SidemenuProps = {
  onSelect: (addr: string) => void
};

function GameItem(props: GameRegistration & SidemenuProps) {
  return <div className="flex flex-col my-4"
    onClick={() => props.onSelect(props.addr)}>
    <h2 className="w-full text-xl underline cursor-pointer hover:scale-[105%] transition-all">{props.title}</h2>
    <h4 className="self-end text-sm text-gray-500">{props.bundle_addr}</h4>
  </div >
}

function Sidemenu(props: SidemenuProps) {
  const helper = useContext(HelperContext);
  const [games, setGames] = useState<GameRegistration[] | undefined>(undefined);

  useEffect(() => {
    if (helper !== undefined) {
      helper.list_games(["DEFAULT_REGISTRATION_ADDRESS"]).then(r => {
        console.log("Games: ", r);
        setGames(r);
      })
    }
  }, [helper]);

  return (
    <div className="p-4">
      <h3 className="font-bold">Demos:</h3>
      {
        games !== undefined ?
          games.map((game) => <GameItem key={game.addr} {...props} {...game} />) :
      "Loading..."
      }
    </div>
  )
}

Sidemenu.contextType = AppHelper;

export default Sidemenu;
