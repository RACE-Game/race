import { IWallet } from './wallet';
import {
  GameAccount,
  GameBundle,
  ServerAccount,
  PlayerProfile,
  VoteType,
  RegistrationAccount,
  INft,
  IToken,
  RegistrationWithGames,
  RecipientAccount,
  EntryType,
} from './accounts';
import { IStorage } from './storage';

export type TransactionResultSuccess<T> = T extends void ? { result: 'ok', } : { result: 'ok', value: T };

export type TransactionResult<T> =
  | TransactionResultSuccess<T>
  | {
    result: 'rejected'
  }
  | {
    result: 'insufficient-funds'
  }
  | {
    result: 'err',
    error: string
  };

export type CreateGameAccountParams = {
  title: string;
  bundleAddr: string;
  tokenAddr: string;
  maxPlayers: number;
  entryType: EntryType;
  recipientAddr: string;
  data: Uint8Array;
};

export type CloseGameAccountParams = {
  gameAddr: string;
};

export type JoinParams = {
  gameAddr: string;
  amount: bigint;
  position: number;
  verifyKey: string;
  createProfileIfNeeded?: boolean;
};

export type DepositParams = {
  playerAddr: string;
  gameAddr: string;
  amount: bigint;
  settleVersion: bigint;
};

export type VoteParams = {
  gameAddr: string;
  voteType: VoteType;
  voterAddr: string;
  voteeAddr: string;
};

export type CreatePlayerProfileParams = {
  nick: string;
  pfp?: string;
};

export type PublishGameParams = {
  uri: string;
  name: string;
  symbol: string;
};

export type CreateRegistrationParams = {
  isPrivate: boolean;
  size: number;
};

export type RegisterGameParams = {
  gameAddr: string;
  regAddr: string;
};

export type UnregisterGameParams = {
  gameAddr: string;
  regAddr: string;
};

export type RecipientClaimParams = {
  recipientAddr: string;
};

export interface ITransport {
  get chain(): string

  createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<TransactionResult<string>>;

  closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<TransactionResult<void>>;

  join(wallet: IWallet, params: JoinParams): Promise<TransactionResult<void>>;

  deposit(wallet: IWallet, params: DepositParams): Promise<TransactionResult<void>>;

  vote(wallet: IWallet, params: VoteParams): Promise<TransactionResult<void>>;

  createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<TransactionResult<void>>;

  publishGame(wallet: IWallet, params: PublishGameParams): Promise<TransactionResult<string>>;

  createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<TransactionResult<string>>;

  registerGame(wallet: IWallet, params: RegisterGameParams): Promise<TransactionResult<void>>;

  unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<TransactionResult<void>>;

  getGameAccount(addr: string): Promise<GameAccount | undefined>;

  getGameBundle(addr: string): Promise<GameBundle | undefined>;

  getPlayerProfile(addr: string): Promise<PlayerProfile | undefined>;

  getServerAccount(addr: string): Promise<ServerAccount | undefined>;

  getRegistration(addr: string): Promise<RegistrationAccount | undefined>;

  getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined>;

  getRecipient(addr: string): Promise<RecipientAccount | undefined>;

  getToken(addr: string): Promise<IToken | undefined>;

  getNft(addr: string, storage?: IStorage): Promise<INft | undefined>;

  listTokens(storage?: IStorage): Promise<IToken[]>;

  listNfts(walletAddr: string, storage?: IStorage): Promise<INft[]>;

  fetchBalances(walletAddr: string, tokenAddrs: string[]): Promise<Map<string, bigint>>;

  recipientClaim(wallet: IWallet, params: RecipientClaimParams): Promise<TransactionResult<void>>;
}
