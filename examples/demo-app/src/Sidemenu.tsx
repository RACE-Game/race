import { AppHelper } from 'race-sdk';
import { Link } from 'react-router-dom';
import { useContext, useState, useEffect } from 'react';
import HelperContext from './helper-context';
import { REG_ADDR } from './constants';

interface GameRegistration {
  addr: string,
  bundle_addr: string,
  title: string,
  reg_time: number
}

function GameItem(props: GameRegistration) {

  let game = undefined;
  switch (props.bundle_addr) {
    case 'CHAT_BUNDLE':
      game = 'chat';
      break;
    case 'RAFFLE_BUNDLE':
      game = 'raffle';
      break;
    case 'DRAW_CARD_BUNDLE':
      game = 'draw-card';
      break;
  }

  return <Link className="flex flex-col my-4"
    to={`${game}/${props.addr}`}>
    <h2 className="w-full text-xl underline cursor-pointer hover:scale-[105%] transition-all">{props.title}</h2>
    <h4 className="self-end text-sm text-gray-500">{props.bundle_addr}</h4>
  </Link>
}

function Sidemenu() {
  const helper = useContext(HelperContext);
  const [games, setGames] = useState<GameRegistration[] | undefined>(undefined);

  useEffect(() => {
    if (helper !== undefined) {
      helper.list_games([REG_ADDR]).then(r => {
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
          games.map((game) => <GameItem key={game.addr} {...game} />) :
          "Loading..."
      }
    </div>
  )
}

Sidemenu.contextType = AppHelper;

export default Sidemenu;
