import { EntryType, INft, IToken } from "./accounts";
import { ConnectionState, Message } from "./connection";
import { GameEvent } from "./events";
import { GameContextSnapshot } from "./game-context-snapshot";
import { TxState } from "./tx-state";

export type Id = number;
export type Ciphertext = Uint8Array;
export type Secret = Uint8Array;
export type Digest = Uint8Array;
export type Fields<T> = Pick<T, keyof T>;

export type GameInfo = {
  gameAddr: string;
  title: string;
  maxPlayers: number;
  minDeposit?: bigint;
  maxDeposit?: bigint;
  entryType: EntryType,
  token: IToken;
  tokenAddr: string;
  bundleAddr: string;
  data: Uint8Array;
  dataLen: number;
};

export type PlayerProfileWithPfp = {
  pfp: INft | undefined,
  addr: string,
  nick: string,
};


export type EventCallbackFunction = (
  context: GameContextSnapshot,
  state: Uint8Array,
  event: GameEvent | undefined,
  isHistory: boolean,
) => void;

export type MessageCallbackFunction = (message: Message) => void;

export type TxStateCallbackFunction = (txState: TxState) => void;

export type ConnectionStateCallbackFunction = (connState: ConnectionState) => void;

export type ProfileCallbackFunction = (id: bigint | undefined, profile: PlayerProfileWithPfp) => void;

export type LoadProfileCallbackFunction = (id: bigint, addr: string) => void;
