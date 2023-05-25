import { nanoid } from 'nanoid';
import {
  CloseGameAccountParams,
  CreateGameAccountParams,
  CreatePlayerProfileParams,
  CreateRegistrationParams,
  DepositParams,
  GameAccount,
  GameBundle,
  INft,
  IToken,
  ITransport,
  IWallet,
  JoinParams,
  PlayerProfile,
  PublishGameParams,
  RegisterGameParams,
  RegistrationAccount,
  ServerAccount,
  UnregisterGameParams,
  VoteParams,
} from '@race-foundation/sdk-core';
import { deserialize } from '@race-foundation/borsh';

interface JoinInstruction {
  playerAddr: string;
  gameAddr: string;
  position: number;
  amount: bigint;
  accessVersion: bigint;
}

interface CreatePlayerProfileInstruction {
  playerAddr: string;
  nick: string;
  pfp?: string;
}

export class FacadeTransport implements ITransport {
  #url: string;

  constructor(url: string = 'http://localhost:12002') {
    this.#url = url;
  }

  createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
    throw new Error('Method not implemented.');
  }
  closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void> {
    throw new Error('Method not implemented.');
  }
  deposit(wallet: IWallet, params: DepositParams): Promise<void> {
    throw new Error('Method not implemented.');
  }
  vote(wallet: IWallet, params: VoteParams): Promise<void> {
    throw new Error('Method not implemented.');
  }
  publishGame(wallet: IWallet, params: PublishGameParams): Promise<string> {
    throw new Error('Method not implemented.');
  }
  createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<string> {
    throw new Error('Method not implemented.');
  }
  registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void> {
    throw new Error('Method not implemented.');
  }
  unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void> {
    throw new Error('Method not implemented.');
  }

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<void> {
    const playerAddr = wallet.walletAddr;
    const ix: CreatePlayerProfileInstruction = { playerAddr, ...params };
    await this.sendInstruction('create_profile', ix);
  }
  async join(wallet: IWallet, params: JoinParams): Promise<void> {
    const playerAddr = wallet.walletAddr;
    const ix: JoinInstruction = { playerAddr, ...params };
    await this.sendInstruction('join', ix);
  }
  async getGameAccount(addr: string): Promise<GameAccount | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_account_info', addr);
    if (data === undefined) return undefined;
    return deserialize(GameAccount, data);
  }
  async getGameBundle(addr: string): Promise<GameBundle | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_game_bundle', addr);
    if (data === undefined) return undefined;
    return deserialize(GameBundle, data);
  }
  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_profile', addr);
    if (data === undefined) return undefined;
    return deserialize(PlayerProfile, data);
  }
  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_server_info', addr);
    if (data === undefined) return undefined;
    return deserialize(ServerAccount, data);
  }
  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_registration_info', addr);
    if (data === undefined) return undefined;
    return deserialize(RegistrationAccount, data);
  }

  async getToken(addr: string): Promise<IToken | undefined> {
    return {
      addr,
      icon: '',
      name: 'Fake Token',
      symbol: 'FT',
      decimals: 0,
    };
  }

  async getNft(_addr: string): Promise<INft | undefined> {
    return undefined;
  }

  async sendInstruction(method: string, ix: any) {
    const reqData = JSON.stringify(
      {
        jsonrpc: '2.0',
        method,
        id: nanoid(),
        params: [ix],
      },
      (_key, value) => (typeof value === 'bigint' ? Number(value) : value)
    );
    const resp = await fetch(this.#url, {
      method: 'POST',
      body: reqData,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    if (!resp.ok) {
      throw new Error('Failed to send instruction: ' + reqData);
    }
  }

  async fetchState(method: string, addr: string): Promise<Uint8Array | undefined> {
    const reqData = JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: nanoid(),
      params: [addr],
    });
    const resp = await fetch(this.#url, {
      method: 'POST',
      body: reqData,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    if (!resp.ok) {
      throw new Error('Failed to fetch data at :' + addr);
    }
    const { result } = await resp.json();
    if (result !== null) {
      return Uint8Array.from(result);
    } else {
      return undefined;
    }
  }

}
