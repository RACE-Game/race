import React from "react";
import { useContext, useEffect, useRef, useState } from "react";
import { useParams } from 'react-router-dom';
import { AppClient, Event } from 'race-sdk';
import { CHAIN_TO_RPC } from "./constants";
import Card from './Card';
import { PlayerProfile } from '@race/sdk-core';
import { ProfileContext } from "./profile-context";
import { LogsContext } from "./logs-context";
import Header from "./Header";
import { useWallet, createTransport } from './integration';
import { useGameContext } from "./App";

type GameStage = "Dealing" | "Betting" | "Reacting" | "Revealing";

interface FormData {
    bet: bigint
}

interface Player {
    addr: string,
    balance: bigint,
    bet: bigint,
}

interface State {
    last_winner: string | null,
    random_id: number,
    players: Player[],
    stage: GameStage,
    bet: bigint,
    blind_bet: bigint,
    min_bet: bigint,
    max_bet: bigint,
}

function renderWaitingPlayers(state: State, profile: PlayerProfile, client: AppClient) {
    let n = state.players.length;
    let canJoin = state.players.find((p) => p.addr == profile.addr) === undefined;
    let onJoin = async () => {
        client.join(1000n);
    };

    return <div className="w-full h-full flex justify-center items-center flex-col">
        <div>Waiting for <span className="font-bold">{2 - n}</span> players to start</div>
        {!canJoin ? null :
            <div className="m-2">
                <button className="border border-black py-2 px-4"
                    onClick={onJoin}>
                    Join
                </button>
            </div>}
    </div>
}

function renderWaitingConnecting() {
    return <div className="w-full h-full font-bold grid place-items-center">
        <div>Connect wallet first!</div>
    </div>
}

function DrawCard() {
    let [state, setState] = useState<State | undefined>(undefined);
    let [form, setForm] = useState<FormData>({ bet: 100n });
    let [setContext] = useState<any | undefined>(undefined);
    let client = useRef<AppClient | undefined>(undefined);
    let { chain } = useGameContext();
    let { addr } = useParams();
    let profile = useContext(ProfileContext);
    let wallet = useWallet(chain);

    let { addLog, clearLog } = useContext(LogsContext);

    const onBet = async () => {
        if (client.current !== undefined) {
            await client.current.submit_event({ 'Bet': Number(form.bet) });
            setForm({ bet: 100n });
        }
    }

    const onCall = async () => {
        if (client.current !== undefined) {
            await client.current.submit_event('Call');
        }
    }

    const onFold = async () => {
        if (client.current !== undefined) {
            await client.current.submit_event('Fold');
        }
    }

    const onChangeBet = (e: React.ChangeEvent<HTMLInputElement>) => {
        let value = e.target.value;
        setForm({ bet: BigInt(value) })
    }

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
                let rpc = CHAIN_TO_RPC[chain];
                let transport = createTransport(chain, rpc);
                let c = await AppClient.try_init(transport, wallet, addr, onEvent);
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

    if (addr === undefined || state === undefined || profile === undefined || client.current === undefined) {
        return renderWaitingConnecting();
    }

    let playerAddr = profile.addr;
    // Render current player at the bottom of the screen and the
    // opponent at the top of the screen.  Render a card back for
    // unrevealed cards.  The hands of current player is always
    // available.  The pot is displayed in the middle of the screen.
    let player = state.players.find((p: Player) => p.addr === playerAddr);
    let opponent = state.players.find((p: Player) => p.addr !== playerAddr);
    let revealed_cards: Record<number, string> = {};
    if (state.random_id > 0) {
        try {
            revealed_cards = client.current.get_revealed(state.random_id);
            console.log("revealed_cards: ", revealed_cards);
        } catch (e) { }
    }

    if (player === undefined || opponent === undefined) {
        return renderWaitingPlayers(state, profile, client.current);
    }

    let [playerPos, opponentPos] = playerAddr == state.players[0]?.addr ? [0, 1] : [1, 0];
    let myHand = revealed_cards[playerPos];
    let myCard = myHand === undefined ? null :
        <Card value={myHand} />;
    let opHand = revealed_cards[opponentPos];
    let opCard = opHand === undefined ?
        <Card value={null} /> :
        <Card value={opHand} />;
    let optionButtons = null;

    if (state.stage === 'Betting' && playerPos === 0) {
        optionButtons = <React.Fragment>
            <input className="m-2 px-4 py-2 border border-black" onChange={onChangeBet} value={form.bet.toString()} />
            <button className="m-2 px-4 py-2 border border-black" onClick={onBet} >Bet</button></React.Fragment>;
    } else if (state.stage === 'Reacting' && playerPos === 1) {
        optionButtons = <React.Fragment>
            <button className="m-2 px-4 py-2 border border-black" onClick={onCall} >Call</button>
            <button className="m-2 px-4 py-2 border border-black" onClick={onFold} >Fold</button>
        </React.Fragment>;
    }

    return (
        <div className="h-full w-full flex flex-col">
            <Header gameAddr={addr} />
            <div className="flex-1 flex flex-col">
                { /* Opponent's hand */}
                <div className="flex-1 grid place-items-center">
                    <div> {opponent.addr + "(" + opponent.balance + ")"} </div>
                    <div> {opCard} </div>
                </div>
                { /* Opponent's bet */}
                <div className="flex-1"> </div>
                { /* Pot */}
                <div className="flex-1"> </div>
                { /* Current player's bet */}
                <div className="flex-1"> </div>
                { /* Current player's hand */}
                <div className="flex-1 grid place-items-center">
                    <div> {myCard} </div>
                    <div> {player.addr + "(" + player.balance + ")"} </div>
                </div>
                <div className="flex-1 flex justify-around items-center">
                    {optionButtons}
                </div>
            </div>
        </div>
    );
}

export default DrawCard;
