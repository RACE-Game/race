import React from "react";
import { useContext, useEffect, useRef, useState } from "react";
import { variant, field, array, struct, option, serialize, deserialize } from '@race-foundation/borsh';
import { useParams } from 'react-router-dom';
import { AppClient, Message, GameContextSnapshot, GameEvent, ICustomEvent, SecretsReady } from '@race-foundation/sdk-core';
import { CHAIN_TO_RPC } from "./constants";
import Card from './Card';
import { ProfileContext } from "./profile-context";
import { LogsContext } from "./logs-context";
import Header from "./Header";
import { useWallet, createTransport } from './integration';
import { useGameContext } from "./App";
import MessagePanel from "./MessagePanel";
import { IMessage } from "./types";

interface FormData {
  bet: bigint
}

abstract class ActionEvent implements ICustomEvent {
  serialize(): Uint8Array {
    return serialize(this);
  }
}

@variant(0)
class Bet extends ActionEvent {
  @field('u64')
  amount: bigint;
  constructor(fields: { amount: bigint }) {
    super();
    this.amount = fields.amount;
  }
}

@variant(1)
class Call extends ActionEvent {
  constructor() { super(); }
}

@variant(2)
class Fold extends ActionEvent {
  constructor() { super(); }
}

class Player {
  @field('u64')
  id!: bigint;
  @field('u64')
  balance!: bigint;
  @field('u64')
  bet!: bigint;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

enum GameStage {
  Dealing = 0,
  Betting,
  Reacting,
  Revealing,
  Ending,
}

class State {
  @field(option('string'))
  lastWinner!: string | undefined;
  @field('usize')
  randomId!: number;
  @field(array(struct(Player)))
  players!: Player[];
  @field('u8')
  stage!: GameStage;
  @field('u64')
  pot!: bigint;
  @field('u64')
  bet!: bigint
  @field('u64')
  blindBet!: bigint;
  @field('u64')
  minBet!: bigint;
  @field('u64')
  maxBet!: bigint;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

function renderWaitingPlayers(state: State, client: AppClient, messages: IMessage[]) {
  let n = state.players.length;
  let canJoin = !client.isInGame();
  let onJoin = async () => {
    client.join({ amount: 1000n });
  };

  return (
    <div className="w-full h-full flex flex-col">
      <div className="flex-1 grid place-items-center">
        <div>Waiting for <span className="font-bold">{2 - n}</span> players to start</div>
        {!canJoin ? null :
          <div className="m-2">
            <button className="border border-black py-2 px-4"
              onClick={onJoin}>
              Join
            </button>
          </div>}
      </div>
      <MessagePanel messages={messages} client={client} />
    </div>
  )
}

function renderWaitingConnecting(client: AppClient | undefined, messages: IMessage[]) {
  return <div className="w-full h-full font-bold flex flex-col">
           <div className="flex-1 grid place-items-center">Connect wallet first!</div>
           {
             client == undefined ? null :
               <MessagePanel messages={messages} client={client} />
           }
         </div>
}

function DrawCard() {
  let [state, setState] = useState<State | undefined>(undefined);
  let [messages, setMessages] = useState<IMessage[]>([]);
  let [form, setForm] = useState<FormData>({ bet: 100n });
  let [revealedCards, setRevealedCards] = useState<Map<number, string> | undefined>(undefined);
  let client = useRef<AppClient | undefined>(undefined);
  let { chain } = useGameContext();
  let { addr } = useParams();
  let profile = useContext(ProfileContext);
  let wallet = useWallet(chain);

  let { addLog, clearLog } = useContext(LogsContext);

  const onBet = async () => {
    if (client.current !== undefined) {
      console.log('onBet', form.bet);
      await client.current.submitEvent(new Bet({ amount: form.bet }));
      setForm({ bet: 100n });
    }
  }

  const onCall = async () => {
    if (client.current !== undefined) {
      await client.current.submitEvent(new Call());
    }
  }

  const onFold = async () => {
    if (client.current !== undefined) {
      await client.current.submitEvent(new Fold());
    }
  }

  const onChangeBet = (e: React.ChangeEvent<HTMLInputElement>) => {
    let value = e.target.value;
    setForm({ bet: BigInt(value) })
  }

  const onMessage = (message: Message) => {
    setMessages(msgs => {
      msgs.push(Object.assign(message, {id: new Date().getTime()}));
      return msgs;
    });
  };

  const onEvent = async (_context: GameContextSnapshot, stateData: Uint8Array, event: GameEvent | undefined) => {
    console.log(stateData);
    const state = deserialize(State, stateData);
    console.log("State:", state);
    if (event !== undefined) {
      addLog(event);
      if (event instanceof SecretsReady && client.current && state) {
        try {
          revealedCards = await client.current.getRevealed(state.randomId);
          console.log("revealed_cards: ", revealedCards);
          setRevealedCards(revealedCards);
        } catch (e) {
          console.error(e);
        }
      }
    }
    setState(state);
  }

  useEffect(() => {
    const initClient = async () => {
      if (profile !== undefined && addr !== undefined) {
        let rpc = CHAIN_TO_RPC[chain];
        let transport = createTransport(chain, rpc);
        let c = await AppClient.initialize({ transport, wallet, gameAddr: addr, onEvent: onEvent, onMessage: onMessage, storage: localStorage });
        client.current = c;
        await c.attachGame();
        console.log("Attached to game");
      }
    };
    initClient();
    return () => {
      clearLog();
      if (client.current) {
        client.current.detach();
      }
    }
  }, [profile, addr]);

  if (addr === undefined || state === undefined || profile === undefined || client.current === undefined) {
    return renderWaitingConnecting(client.current, messages);
  }

  let playerId = client.current.playerId
  // Render current player at the bottom of the screen and the
  // opponent at the top of the screen.  Render a card back for
  // unrevealed cards.  The hands of current player is always
  // available.  The pot is displayed in the middle of the screen.
  let player = state.players.find((p: Player) => p.id === playerId);
  let opponent = state.players.find((p: Player) => p.id !== playerId);

  console.log('client', client.current);
  console.log('state', state);
  console.log('playerId', playerId);
  console.log('player', player);
  console.log('opponent', opponent);

  if (player === undefined || opponent === undefined) {
    return renderWaitingPlayers(state, client.current, messages);
  }

  let [playerPos, opponentPos] = playerId == state.players[0]?.id ? [0, 1] : [1, 0];

  let myHand = revealedCards?.get(playerPos);
  let myCard = myHand === undefined ? null :
    <Card value={myHand} />;
  let opHand = revealedCards?.get(opponentPos);
  let opCard = opHand === undefined ?
    <Card value={null} /> :
    <Card value={opHand} />;
  let optionButtons = null;

  if (state.stage === GameStage.Betting && playerPos === 0) {
    optionButtons = <React.Fragment>
                      <input className="m-2 px-4 py-2 border border-black" onChange={onChangeBet} value={form.bet.toString()} />
                      <button className="m-2 px-4 py-2 border border-black" onClick={onBet} >Bet</button></React.Fragment>;
  } else if (state.stage === GameStage.Reacting && playerPos === 1) {
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
          <div> {opponent.id + "(" + opponent.balance + ")"} </div>
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
          <div> {player.id + "(" + player.balance + ")"} </div>
        </div>
        <div className="flex-1 flex justify-around items-center">
          {optionButtons}
        </div>
      </div>
      <MessagePanel messages={messages} client={client.current} />
    </div>
  );
}

export default DrawCard;
