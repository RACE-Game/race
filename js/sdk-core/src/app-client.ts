import {
  Connection,
  IConnection,
} from './connection';
import { GameContext } from './game-context';
import { ITransport, TransactionResult } from './transport';
import { IWallet } from './wallet';
import { InitAccount } from './init-account';
import { Handler } from './handler';
import { Encryptor, IEncryptor } from './encryptor';
import { SdkError } from './error';
import { Client } from './client';
import { IStorage, getTtlCache, setTtlCache } from './storage';
import { DecryptionCache } from './decryption-cache';
import { ProfileLoader } from './profile-loader';
import { BaseClient } from './base-client';
import { EntryTypeCash, EntryTypeDisabled, GameAccount, GameBundle, IToken } from './accounts';
import { ConnectionStateCallbackFunction, EventCallbackFunction, GameInfo, MessageCallbackFunction, TxStateCallbackFunction, PlayerProfileWithPfp, ProfileCallbackFunction, ErrorCallbackFunction } from './types';
import { SubClient } from './sub-client';

const BUNDLE_CACHE_TTL = 3600 * 365;

export type AppClientInitOpts = {
  transport: ITransport;
  wallet: IWallet;
  gameAddr: string;
  onProfile: ProfileCallbackFunction;
  onEvent: EventCallbackFunction;
  onMessage?: MessageCallbackFunction;
  onTxState?: TxStateCallbackFunction;
  onError?: ErrorCallbackFunction;
  onConnectionState?: ConnectionStateCallbackFunction;
  storage?: IStorage;
};

export type SubClientInitOpts = {
  subId: number;
  gameAddr: string;
  onEvent: EventCallbackFunction;
  onMessage?: MessageCallbackFunction;
  onTxState?: TxStateCallbackFunction;
  onError?: ErrorCallbackFunction;
  onConnectionState?: ConnectionStateCallbackFunction;
};

export type JoinOpts = {
  amount: bigint;
  position?: number;
  createProfileIfNeeded?: boolean;
};

export type AppClientCtorOpts = {
  gameAddr: string;
  handler: Handler;
  wallet: IWallet;
  client: Client;
  transport: ITransport;
  connection: IConnection;
  gameContext: GameContext;
  onEvent: EventCallbackFunction;
  onMessage: MessageCallbackFunction | undefined;
  onTxState: TxStateCallbackFunction | undefined;
  onConnectionState: ConnectionStateCallbackFunction | undefined;
  onError: ErrorCallbackFunction | undefined;
  encryptor: IEncryptor;
  info: GameInfo;
  decryptionCache: DecryptionCache;
  profileLoader: ProfileLoader;
  storage: IStorage | undefined;
  endpoint: string;
};

export class AppClient extends BaseClient {
  __profileLoader: ProfileLoader;
  __storage?: IStorage;
  __endpoint: string;

  constructor(opts: AppClientCtorOpts) {
    super({
      onLoadProfile: (id, addr) => opts.profileLoader.load(id, addr),
      ...opts
    });
    this.__profileLoader = opts.profileLoader;
    this.__storage = opts.storage;
    this.__endpoint = opts.endpoint;
  }

  static async initialize(opts: AppClientInitOpts): Promise<AppClient> {
    const { transport, wallet, gameAddr, onEvent, onMessage, onTxState, onConnectionState, onError, onProfile, storage } = opts;

    console.groupCollapsed('AppClient initialization');
    try {
      const playerAddr = wallet.walletAddr;
      const encryptor = await Encryptor.create(playerAddr, storage);
      const gameAccount = await transport.getGameAccount(gameAddr);
      if (gameAccount === undefined) {
        throw SdkError.gameAccountNotFound(gameAddr);
      }
      console.log('Game account:', gameAccount);

      const bundleCacheKey = `BUNDLE__${transport.chain}_${gameAccount.bundleAddr}`;

      const gameBundle = await getGameBundle(transport, storage, bundleCacheKey, gameAccount.bundleAddr);

      const transactorAddr = gameAccount.transactorAddr;
      if (transactorAddr === undefined) {
        throw SdkError.gameNotServed(gameAddr);
      }
      console.log('Transactor address:', transactorAddr);
      const transactorAccount = await transport.getServerAccount(transactorAddr);
      if (transactorAccount === undefined) {
        throw SdkError.transactorAccountNotFound(transactorAddr);
      }
      const decryptionCache = new DecryptionCache();
      const endpoint = transactorAccount.endpoint;
      console.log('Transactor endpoint:', endpoint);
      const connection = Connection.initialize(gameAddr, playerAddr, endpoint, encryptor);
      const client = new Client(playerAddr, encryptor, connection);
      const handler = await Handler.initialize(gameBundle, encryptor, client, decryptionCache);
      const gameContext = new GameContext(gameAccount);
      const token = await transport.getToken(gameAccount.tokenAddr);
      if (token === undefined) {
        throw SdkError.tokenNotFound(gameAccount.tokenAddr);
      }
      const info = makeGameInfo(gameAccount, token);
      const profileLoader = new ProfileLoader(transport, storage, onProfile);
      profileLoader.start();

      return new AppClient({
        gameAddr,
        handler,
        wallet,
        client,
        transport,
        connection,
        gameContext,
        onEvent,
        onMessage,
        onTxState,
        onConnectionState,
        onError,
        encryptor,
        info,
        decryptionCache,
        profileLoader,
        storage,
        endpoint,
      });
    } finally {
      console.groupEnd();
    }
  }

  async subClient(opts: SubClientInitOpts): Promise<SubClient> {
    try {
      const { subId, onEvent, onMessage, onTxState, onConnectionState, onError } = opts;

      const addr = `${this.__gameAddr}:${subId.toString()}`;

      console.groupCollapsed(`SubClient initialization, id: ${subId}`);

      const subGame = this.__gameContext.findSubGame(subId);

      if (subGame === undefined) {
        console.warn('Game context:', this.__gameContext);
        throw SdkError.invalidSubId(subId);
      }

      const bundleAddr = subGame.bundleAddr;

      const bundleCacheKey = `BUNDLE__${this.__transport.chain}_${bundleAddr}`;

      const decryptionCache = new DecryptionCache();
      const playerAddr = this.__wallet.walletAddr;
      const connection = Connection.initialize(addr, playerAddr, this.__endpoint, this.__encryptor);
      const client = new Client(playerAddr, this.__encryptor, connection);
      const gameBundle = await getGameBundle(this.__transport, this.__storage, bundleCacheKey, bundleAddr);
      const handler = await Handler.initialize(gameBundle, this.__encryptor, client, decryptionCache);
      const gameContext = this.__gameContext.subContext(subGame);
      const initAccount = new InitAccount({
        data: subGame.initData,
        players: subGame.players,
        maxPlayers: 0,
        checkpoint: subGame.checkpoint,
        entryType: new EntryTypeDisabled({}),
      });

      return new SubClient({
        gameAddr: addr,
        wallet: this.__wallet,
        transport: this.__transport,
        encryptor: this.__encryptor,
        onEvent,
        onMessage,
        onTxState,
        onConnectionState,
        onError,
        handler,
        connection,
        client,
        info: this.__info,
        decryptionCache,
        gameContext,
        subId,
        initAccount,
      });
    } finally {
      console.groupEnd();
    }
  }


  /**
   * Get player profile by its wallet address.
   */
  getProfile(id: bigint): Promise<PlayerProfileWithPfp | undefined>
  getProfile(addr: string): Promise<PlayerProfileWithPfp | undefined>
  async getProfile(idOrAddr: string | bigint): Promise<PlayerProfileWithPfp | undefined> {
    let addr: string = ''
    if (typeof idOrAddr === 'bigint') {
      addr = this.__gameContext.idToAddr(idOrAddr);
    } else {
      addr = idOrAddr
    }
    return this.__profileLoader.getProfile(addr);
  }

  makeSubGameAddr(subId: number): string {
    return `${this.__gameAddr}:${subId}`;
  }

  /**
   * Join game.
   */
  async join(params: JoinOpts): Promise<TransactionResult<void>> {
    const gameAccount = await this.__transport.getGameAccount(this.gameAddr);
    if (gameAccount === undefined) {
      throw new Error('Game account not found');
    }
    const playersCount = gameAccount.players.length;
    if (gameAccount.maxPlayers <= playersCount) {
      throw new Error(`Game is full, current number of players: ${playersCount}`);
    }
    let position: number | undefined = params.position;
    if (position === undefined) {
      for (let i = 0; i < gameAccount.maxPlayers; i++) {
        if (gameAccount.players.find(p => p.position === i) === undefined) {
          position = i;
          break;
        }
      }
    }
    if (position === undefined) {
      throw new Error(`The position has been taken: ${params.position}`);
    }

    const publicKey = await this.__encryptor.exportPublicKey();

    let createProfile = false;
    if (params.createProfileIfNeeded) {
      const p = await this.__transport.getPlayerProfile(this.playerAddr);
      if (p === undefined) {
        createProfile = true;
        console.log('No profile account found, will create a new one.')
      }
    }

    return await this.__transport.join(this.__wallet, {
      gameAddr: this.gameAddr,
      amount: params.amount,
      accessVersion: gameAccount.accessVersion,
      position,
      verifyKey: publicKey.ec,
      createProfile,
    });
  }

}


// Miscellaneous

export async function getGameBundle(transport: ITransport, storage: IStorage | undefined, bundleCacheKey: string, bundleAddr: string): Promise<GameBundle> {
  let gameBundle: GameBundle | undefined;
  if (storage !== undefined) {
    gameBundle = getTtlCache(storage, bundleCacheKey);
    console.log('Use game bundle from cache:', gameBundle);
    if (gameBundle !== undefined) {
      Object.assign(gameBundle, { data: Uint8Array.of() })
    }
  }
  if (gameBundle === undefined) {
    gameBundle = await transport.getGameBundle(bundleAddr);
    console.log('Game bundle:', gameBundle);
    if (gameBundle !== undefined && storage !== undefined && gameBundle.data.length === 0) {
      setTtlCache(storage, bundleCacheKey, gameBundle, BUNDLE_CACHE_TTL);
    }
  }
  if (gameBundle === undefined) {
    throw SdkError.gameBundleNotFound(bundleAddr);
  }
  return gameBundle;
}

export function makeGameInfo(gameAccount: GameAccount, token: IToken): GameInfo {
  const info: GameInfo = {
    gameAddr: gameAccount.addr,
    title: gameAccount.title,
    entryType: gameAccount.entryType,
    maxPlayers: gameAccount.maxPlayers,
    tokenAddr: gameAccount.tokenAddr,
    bundleAddr: gameAccount.bundleAddr,
    data: gameAccount.data,
    dataLen: gameAccount.dataLen,
    token,
  };

  if (gameAccount.entryType instanceof EntryTypeCash) {
    info.minDeposit = gameAccount.entryType.minDeposit;
    info.maxDeposit = gameAccount.entryType.maxDeposit;
  }

  return info;
}
