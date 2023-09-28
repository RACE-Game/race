import { makeid } from './utils';
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
  RecipientAccount,
  RecipientClaimParams,
  RegisterGameParams,
  RegistrationAccount,
  RegistrationWithGames,
  ServerAccount,
  UnregisterGameParams,
  VoteParams,
  EntryType,
  EntryTypeCash,
  IStorage,
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

interface CreateGameAccountInstruction {
  walletAddr: string;
  gameAddr: string;
  title: string;
  bundleAddr: string;
  tokenAddr: string;
  maxPlayers: number;
  minDeposit?: bigint;
  maxDeposit?: bigint;
  soltId?: number,
  ticketPrice?: bigint,
  collection?: string,
  data: number[];
}

const tokenMap: Record<string, IToken> = {
  'FACADE_NATIVE': {
    name: 'Native Token',
    symbol: 'NATIVE',
    decimals: 9,
    icon: 'https://arweave.net/SH106hrChudKjQ_c6e6yd0tsGUbFIScv2LL6Dp-LDiI',
    addr: 'FACADE_NATIVE',
  },
  'FACADE_USDT': {
    name: 'Tether USD',
    symbol: 'USDT',
    decimals: 6,
    icon: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.svg',
    addr: 'FACADE_USDT',
  },
  'FACADE_USDC': {
    name: 'USD Coin',
    symbol: 'USDC',
    decimals: 6,
    icon: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png',
    addr: 'FACADE_USDC',
  },
  'FACADE_RACE': {
    name: 'Race Protocol',
    symbol: 'RACE',
    decimals: 9,
    icon: 'https://raw.githubusercontent.com/NutsPokerTeam/token-list/main/assets/mainnet/RACE5fnTKB9obGtCusArTQ6hhdNXAtf3HarvJM17rxJ/logo.svg',
    addr: 'FACADE_RACE',
  }
};


export class FacadeTransport implements ITransport {
  #url: string;

  constructor(url: string = 'http://localhost:12002') {
    this.#url = url;
  }

  get chain() {
    return 'Facade'
  }

  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
    const walletAddr = wallet.walletAddr;
    const gameAddr = makeid(16);
    const data = [...params.data];
    const ix: CreateGameAccountInstruction = { walletAddr, gameAddr, ...params, ...params.entryType, data };
    await this.sendInstruction('create_account', ix);
    return gameAddr;
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
  recipientClaim(wallet: IWallet, params: RecipientClaimParams): Promise<void> {
    throw new Error('Method not implemented.');
  }

  async listTokens(_storage?: IStorage): Promise<IToken[]> {
    return Object.values(tokenMap);
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
    const data: Uint8Array | undefined = await this.fetchState('get_account_info', [addr]);
    if (data === undefined) return undefined;
    return deserialize(GameAccount, data);
  }
  async getGameBundle(addr: string): Promise<GameBundle | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_game_bundle', [addr]);
    if (data === undefined) return undefined;
    return deserialize(GameBundle, data);
  }
  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_profile', [addr]);
    if (data === undefined) return undefined;
    return deserialize(PlayerProfile, data);
  }
  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_server_info', [addr]);
    if (data === undefined) return undefined;
    return deserialize(ServerAccount, data);
  }
  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_registration_info', [addr]);
    if (data === undefined) return undefined;
    return deserialize(RegistrationAccount, data);
  }

  async getRecipient(addr: string): Promise<RecipientAccount | undefined> {
    return undefined;
  }

  async getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined> {
    const data: Uint8Array | undefined = await this.fetchState('get_registration_info', [addr]);
    if (data === undefined) return undefined;
    const regAccount = deserialize(RegistrationAccount, data);
    const promises = regAccount.games.map(async g => {
      return await this.getGameAccount(g.addr);
    });
    const games = await Promise.all(promises);
    return new RegistrationWithGames({
      ...regAccount,
      games,
    });
  }

  async getToken(addr: string): Promise<IToken | undefined> {
    return tokenMap[addr];
  }

  async fetchBalances(walletAddr: string, tokenAddrs: string[]): Promise<Map<string, bigint>> {
    let ret = new Map();
    for (const addr of tokenAddrs) {
      const data = await this.fetchState('get_balance', [walletAddr, addr]);
      if (data !== undefined) {
        const view = new DataView(data.buffer);
        const balance = view.getBigUint64(0, true);
        ret.set(addr, balance);
      }
    }
    return ret;
  }

  async getNft(_addr: string, _storage?: IStorage): Promise<INft | undefined> {
    return undefined;
  }

  async listNfts(_walletAddr: string, _storage?: IStorage): Promise<INft[]> {
    return [];
  }

  async sendInstruction(method: string, ix: any) {
    const reqData = JSON.stringify(
      {
        jsonrpc: '2.0',
        method,
        id: makeid(16),
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

  async fetchState(method: string, params: any): Promise<Uint8Array | undefined> {
    const reqData = JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: makeid(16),
      params,
    });
    const resp = await fetch(this.#url, {
      method: 'POST',
      body: reqData,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    if (!resp.ok) {
      throw new Error('Failed to fetch data at :' + params);
    }
    const { result } = await resp.json();
    if (result !== null) {
      return Uint8Array.from(result);
    } else {
      return undefined;
    }
  }
}
