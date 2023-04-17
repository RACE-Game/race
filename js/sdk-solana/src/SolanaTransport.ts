import { Connection, PublicKey } from '@solana/web3.js';
import {
  IWallet,
  ITransport,
  CreateGameAccountParams,
  CloseGameAccountParams,
  JoinParams,
  DepositParams,
  VoteParams,
  CreatePlayerProfileParams,
  PublishGameParams,
  CreateRegistrationParams,
  RegisterGameParams,
  UnregisterGameParams,
  GameAccount,
  GameBundle,
  PlayerProfile,
  ServerAccount,
  RegistrationAccount,
} from 'race-sdk-core';

const PROGRAM_ID = new PublicKey('8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu');

class SolanaTransport implements ITransport {
  #conn: Connection;

  constructor(endpoint: string) {
    this.#conn = new Connection(endpoint, 'finalized');
  }

  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
    return '';
  }

  async closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void> {}

  async join(wallet: IWallet, params: JoinParams): Promise<void> {}

  async deposit(wallet: IWallet, params: DepositParams): Promise<void> {}

  async publishGame(wallet: IWallet, params: PublishGameParams): Promise<string> {
    return '';
  }

  async vote(wallet: IWallet, params: VoteParams): Promise<void> {}

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<string> {
    return '';
  }

  async createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<string> {
    return '';
  }

  async registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void> {}

  async unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void> {}

  async getGameAccount(addr: string): Promise<GameAccount | undefined> {
    return undefined;
  }

  async getGameBundle(addr: string): Promise<GameBundle | undefined> {
    return undefined;
  }

  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    return undefined;
  }

  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    return undefined;
  }

  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    return undefined;
  }
}

export default SolanaTransport;
