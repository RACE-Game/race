import { useContext, useEffect, useState } from "react";
import { useParams } from 'react-router-dom';
import { AppClient, GameEvent } from '@race/sdk-core';
import { CHAIN_TO_RPC } from "./constants";
import { ProfileContext } from "./profile-context";
import { LogsContext } from "./logs-context";
import { useGameContext } from "./App";
import { createTransport, useWallet } from './integration';
import { deserialize, field, option, struct, array } from '@race/borsh';
import { GameContextSnapshot } from "@race/sdk-core/lib/types/game-context-snapshot";

interface IPlayer {
    addr: string;
    balance: bigint;
}

interface IState {
    lastWinner: string | undefined;
    players: IPlayer[];
    randomId: number;
    draw_time: bigint;
}

class Player {
    @field('string')
    addr!: string;
    @field('u64')
    balance!: bigint;
    constructor(fields: IPlayer) {
        Object.assign(this, fields);
    }
}

class State {
    @field(option('string'))
    lastWinner: string | undefined;
    @field(array(struct(Player)))
    players!: IPlayer[];
    @field('u64')
    randomId!: number;
    @field('u64')
    draw_time!: bigint;
    constructor(fields: IState) {
        Object.assign(this, fields);
    }
}

function Winner(props: { settleVersion: bigint, lastWinner: string | undefined }) {

    const [fade, setFade] = useState(false);

    useEffect(() => {
        setFade(false);
        setTimeout(() => setFade(true), 5000)
    }, [props.settleVersion]);

    if (props.lastWinner) {
        return <div className={
            `bg-black text-white text-lg p-4 text-center animate-bounce transition-opacity duration-[3500ms]
       ${fade ? "opacity-0" : "opacity-100"}`}>
            Winner: {props.lastWinner}
        </div>
    } else {
        return <div></div>
    }
}

function Raffle() {
    let [state, setState] = useState<IState | undefined>(undefined);
    let [context, setContext] = useState<GameContextSnapshot | undefined>(undefined);
    let [client, setClient] = useState<AppClient | undefined>(undefined);
    let { addr } = useParams();
    let { chain } = useGameContext();
    let profile = useContext(ProfileContext);
    let wallet = useWallet(chain);
    let { addLog } = useContext(LogsContext);

    // Game event handler
    const onEvent = (context: GameContextSnapshot, stateData: Uint8Array, event: GameEvent | undefined) => {
        const state = deserialize(State, stateData);
        if (event !== undefined) {
            addLog(event);
        }
        setContext(context);
        setState(state);
    }

    // Button callback to join the raffle
    const onJoin = async () => {
        if (client !== undefined) {
            await client.join({ amount: 10n });
        }
    }

    // Initialize app client
    useEffect(() => {
        const initClient = async () => {
            if (profile !== undefined && addr !== undefined) {
                let rpc = CHAIN_TO_RPC[chain];
                let transport = createTransport(chain, rpc);
                let client = await AppClient.initialize({ transport, wallet, gameAddr: addr, callback: onEvent });
                setClient(client);
                await client.attachGame();
            }
        };
        initClient();
    }, [profile, addr]);

    if (state === undefined || context === undefined) {
        return <div className="h-full w-full grid place-items-center">
            <svg className="animate-spin h-5 w-5 mr-3 border border-black" viewBox="0 0 24 24"></svg>
        </div>
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
                    context.players.map((p, i) => {
                        return <div key={i} className="m-2 p-2 border border-black">
                            {p.profile?.nick} @ [{p.addr}]
                        </div>
                    })
                }

                <div className="flex-1"></div>

                <Winner
                    lastWinner={state.lastWinner}
                    settleVersion={context.settleVersion} />
            </div>
        );
    }
}


export default Raffle;
