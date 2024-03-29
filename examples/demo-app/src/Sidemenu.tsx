import { Link } from 'react-router-dom';
import { useContext, useState, useEffect } from 'react';
import { HelperContext } from './helper-context';
import { Chain } from './types';
import { CHAIN_ADDR_GAME_MAPPING, CHAIN_TO_REG_ADDR } from './constants';
import { AppHelper, GameAccount } from '@race-foundation/sdk-core';

interface SidemenuProps {
    chain: Chain,
}

function GameItem(props: GameAccount & { chain: Chain }) {

    const game = CHAIN_ADDR_GAME_MAPPING[props.chain][props.bundleAddr]
    return <Link className="flex flex-col my-4"
        to={`${game}/${props.addr}`}>
        <h2 className="w-full text-xl underline cursor-pointer hover:scale-[105%] transition-all">{props.title}</h2>
        <h4 className="self-end text-sm text-gray-500">{props.bundleAddr}</h4>
    </Link>
}

function Sidemenu(props: SidemenuProps) {
    const { chain } = props;
    const helper = useContext(HelperContext);
    const [games, setGames] = useState<GameAccount[] | undefined>(undefined);

    useEffect(() => {
        if (helper !== undefined) {
            console.info("Fetch registration", [CHAIN_TO_REG_ADDR[chain]]);
            helper.listGames([CHAIN_TO_REG_ADDR[chain]]).then(r => {
                console.log("Games:", r);
                setGames(r);
            })
        }
    }, [helper]);

    return (
        <div className="h-full w-full flex flex-col overflow-scroll-y border border-gray-500 p-4">
            <h3 className="uppercase font-bold">Demos</h3>
            {
                games !== undefined ?
                    games.map((game) => <GameItem key={game.addr} chain={chain} {...game} />) :
                    "Loading..."
            }
        </div>
    )
}

export default Sidemenu;
