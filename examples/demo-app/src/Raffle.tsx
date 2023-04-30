import { useContext, useEffect, useState } from "react";
import { useNavigate, useParams } from 'react-router-dom';
import { AppClient, Event } from 'race-sdk';
import { CHAIN_TO_RPC } from "./constants";
import { ProfileContext } from "./profile-context";
import { LogsContext } from "./logs-context";
import { useGameContext } from "./App";
import { createTransport, useWallet } from './integration';
import { HelperContext } from "./helper-context";

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
    let [client, setClient] = useState<AppClient | undefined>(undefined);
    let { addr } = useParams();
    let { chain } = useGameContext();
    let profile = useContext(ProfileContext);
    let wallet = useWallet(chain);
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
            console.log(wallet);
            await client.join(1000000000n);
        }
    }

    // Initialize app client
    useEffect(() => {
        const initClient = async () => {
            if (profile !== undefined && addr !== undefined) {
                let rpc = CHAIN_TO_RPC[chain];
                let transport = createTransport(chain, rpc);
                let client = await AppClient.try_init(transport, wallet, addr, onEvent);
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
                        state.draw_time ? new Date(Number(state.draw_time)).toLocaleTimeString() : "N/A"
                    }
                </div>
                <div>Players:</div>
                {
                    context.players.map((p: any, i: number) => {
                        return <div key={i} className="m-2 p-2 border border-black">
                            {p.addr}
                        </div>
                    })
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
