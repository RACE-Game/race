import { AppHelper } from 'race-sdk';
import { Link } from 'react-router-dom';
import { useContext, useState, useEffect } from 'react';
import HelperContext from './helper-context';
import { Chain } from './types';
import { CHAIN_TO_REG_ADDR, CHAIN_TO_RPC } from './constants';

interface GameRegistration {
    addr: string,
    bundle_addr: string,
    title: string,
    reg_time: number
}

interface SidemenuProps {
    chain: Chain,
}

function GameItem(props: GameRegistration) {
    let game = undefined;
    switch (props.bundle_addr) {
        case 'CHAT_BUNDLE_ADDRESS':
            game = 'chat';
            break;
        case 'RAFFLE_BUNDLE_ADDRESS':
            game = 'raffle';
            break;
    }

    return <Link className="flex flex-col my-4"
        to={`${game}/${props.addr}`}>
        <h2 className="w-full text-xl underline cursor-pointer hover:scale-[105%] transition-all">{props.title}</h2>
        <h4 className="self-end text-sm text-gray-500">{props.bundle_addr}</h4>
    </Link>
}

function Sidemenu(props: SidemenuProps) {
    const { chain } = props;
    const helper = useContext(HelperContext);
    const [games, setGames] = useState<GameRegistration[] | undefined>(undefined);

    useEffect(() => {
        if (helper !== undefined) {
            console.info("Fetch registration", [CHAIN_TO_REG_ADDR[chain]]);
            helper.list_games([CHAIN_TO_REG_ADDR[chain]]).then(r => {
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
