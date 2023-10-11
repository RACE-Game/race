import {
  BroadcastFrameEvent,
  BroadcastFrameMessage,
  BroadcastFrameTxState,
  Connection,
  ConnectionState,
  IConnection,
  Message,
  SubmitEventParams,
  SubmitMessageParams,
  SubscribeEventParams,
} from './connection';
import { GameContext } from './game-context';
import { GameContextSnapshot } from './game-context-snapshot';
import { ITransport, TransactionResult } from './transport';
import { IWallet } from './wallet';
import { Handler, InitAccount } from './handler';
import { Encryptor, IEncryptor } from './encryptor';
import { SdkError } from './error';
import { EntryType, EntryTypeCash, GameAccount, GameBundle, IToken, PlayerProfile } from './accounts';
import { TxState } from './tx-state';
import { Client } from './client';
import { Custom, GameEvent, ICustomEvent } from './events';
import { ProfileCache } from './profile-cache';
import { IStorage, getTtlCache, setTtlCache } from './storage';
import { DecryptionCache } from './decryption-cache';

const BUNDLE_CACHE_TTL = 3600 * 365;

export type EventCallbackFunction = (
  context: GameContextSnapshot,
  state: Uint8Array,
  event: GameEvent | undefined
) => void;
export type MessageCallbackFunction = (message: Message) => void;
export type TxStateCallbackFunction = (txState: TxState) => void;
export type OnConnectionStateCallbackFunction = (connState: ConnectionState) => void;

export type AppClientInitOpts = {
  transport: ITransport;
  wallet: IWallet;
  gameAddr: string;
  onEvent: EventCallbackFunction;
  onMessage?: MessageCallbackFunction;
  onTxState?: TxStateCallbackFunction;
  onConnectionState?: OnConnectionStateCallbackFunction;
  storage?: IStorage;
};

export type JoinOpts = {
  amount: bigint;
  position?: number;
};

export type GameInfo = {
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

export class AppClient {
  #gameAddr: string;
  #handler: Handler;
  #wallet: IWallet;
  #client: Client;
  #transport: ITransport;
  #connection: IConnection;
  #gameContext: GameContext;
  #onEvent: EventCallbackFunction;
  #onMessage?: MessageCallbackFunction;
  #onTxState?: TxStateCallbackFunction;
  #onConnectionState?: OnConnectionStateCallbackFunction;
  #encryptor: IEncryptor;
  #profileCaches: ProfileCache;
  #info: GameInfo;
  #decryptionCache: DecryptionCache;

  constructor(
    gameAddr: string,
    handler: Handler,
    wallet: IWallet,
    client: Client,
    transport: ITransport,
    connection: IConnection,
    gameContext: GameContext,
    onEvent: EventCallbackFunction,
    onMessage: MessageCallbackFunction | undefined,
    onTxState: TxStateCallbackFunction | undefined,
    onConnectionState: OnConnectionStateCallbackFunction | undefined,
    encryptor: IEncryptor,
    info: GameInfo,
    decryptionCache: DecryptionCache,
  ) {
    this.#gameAddr = gameAddr;
    this.#handler = handler;
    this.#wallet = wallet;
    this.#client = client;
    this.#transport = transport;
    this.#connection = connection;
    this.#gameContext = gameContext;
    this.#onEvent = onEvent;
    this.#onMessage = onMessage;
    this.#onTxState = onTxState;
    this.#onConnectionState = onConnectionState;
    this.#encryptor = encryptor;
    this.#profileCaches = new ProfileCache(transport);
    this.#info = info;
    this.#decryptionCache = decryptionCache;
  }

  static async initialize(opts: AppClientInitOpts): Promise<AppClient> {
    const { transport, wallet, gameAddr, onEvent, onMessage, onTxState, onConnectionState, storage } = opts;
    console.group('AppClient initialization');
    try {
      const playerAddr = wallet.walletAddr;
      const encryptor = await Encryptor.create(playerAddr, storage);
      const gameAccount = await transport.getGameAccount(gameAddr);
      if (gameAccount === undefined) {
        throw SdkError.gameAccountNotFound(gameAddr);
      }
      console.log('Game account:', gameAccount);

      // Fetch game bundle
      // The bundle can be considered as immutable, so we use cache whenever possible
      const bundleCacheKey = `BUNDLE__${transport.chain}_${gameAccount.bundleAddr}`;

      let gameBundle: GameBundle | undefined;
      if (storage !== undefined) {
        gameBundle = getTtlCache(storage, bundleCacheKey);
        console.log('Use game bundle from cache:', gameBundle);
        if (gameBundle !== undefined) {
          Object.assign(gameBundle, { data: Uint8Array.of() })
        }
      }
      if (gameBundle === undefined) {
        gameBundle = await transport.getGameBundle(gameAccount.bundleAddr);
        console.log('Game bundle:', gameBundle);
        if (gameBundle !== undefined && storage !== undefined && gameBundle.data.length === 0) {
          setTtlCache(storage, bundleCacheKey, gameBundle, BUNDLE_CACHE_TTL);
        }
      }
      if (gameBundle === undefined) {
        throw SdkError.gameBundleNotFound(gameAccount.bundleAddr);
      }

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
      console.log('Transactor endpoint:', transactorAccount.endpoint);
      const connection = Connection.initialize(gameAddr, playerAddr, transactorAccount.endpoint, encryptor);
      const client = new Client(playerAddr, gameAddr, encryptor, connection);
      const handler = await Handler.initialize(gameBundle, encryptor, client, decryptionCache);
      const gameContext = new GameContext(gameAccount);
      gameContext.applyCheckpoint(gameContext.checkpointAccessVersion, gameContext.settleVersion);
      const token = await transport.getToken(gameAccount.tokenAddr);
      if (token === undefined) {
        throw SdkError.tokenNotFound(gameAccount.tokenAddr);
      }
      const info: GameInfo = {
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

      return new AppClient(
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
        encryptor,
        info,
        decryptionCache,
      );
    } finally {
      console.groupEnd();
    }
  }

  get playerAddr() {
    return this.#wallet.walletAddr;
  }

  get gameAddr() {
    return this.#gameAddr;
  }

  get gameContext(): GameContext {
    return this.#gameContext;
  }

  /**
   * Get player profile by its wallet address.
   */
  async getProfile(addr: string): Promise<PlayerProfile | undefined> {
    return await this.#transport.getPlayerProfile(addr);
  }

  async invokeEventCallback(event: GameEvent | undefined) {
    const snapshot = new GameContextSnapshot(this.#gameContext);
    await this.#profileCaches.injectProfiles(snapshot);
    const state = this.#gameContext.handlerState;
    this.#onEvent(snapshot, state, event);
  }

  async __initializeState(gameAccount: GameAccount): Promise<void> {
    const initAccount = InitAccount.createFromGameAccount(gameAccount, this.gameContext.accessVersion, this.gameContext.settleVersion);
    console.log('Initialize state with', initAccount);
    await this.#handler.initState(this.#gameContext, initAccount);
    await this.invokeEventCallback(undefined);
  }

  async __getGameAccount(): Promise<GameAccount> {
    while (true) {
      try {
        const gameAccount = await this.#transport.getGameAccount(this.gameAddr);
        if (gameAccount === undefined) continue;
        console.log('Game account', gameAccount);
        return gameAccount;
      } catch (e: any) {
        console.warn(e, 'Failed to fetch game account, will retry in 3s');
        await new Promise(r => setTimeout(r, 3000));
        continue;
      }
    }

  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    await this.#client.attachGame();
    const sub = this.#connection.subscribeEvents();
    const gameAccount = await this.__getGameAccount();
    this.#gameContext = new GameContext(gameAccount);
    this.#gameContext.applyCheckpoint(gameAccount.checkpointAccessVersion, this.#gameContext.settleVersion);
    await this.#connection.connect(new SubscribeEventParams({ settleVersion: this.#gameContext.settleVersion }));
    await this.__initializeState(gameAccount);

    for await (const frame of sub) {
      if (frame instanceof BroadcastFrameMessage) {
        console.group('Receive message');
        try {
          if (this.#onMessage !== undefined) {
            const { message } = frame;
            this.#onMessage(message);
          }
        } finally {
          console.groupEnd();
        }
      } else if (frame instanceof BroadcastFrameTxState) {
        console.group('Receive tx state');
        try {
          if (this.#onTxState !== undefined) {
            const { txState } = frame;
            this.#onTxState(txState);
          }
        } finally {
          console.groupEnd();
        }
      } else if (frame instanceof BroadcastFrameEvent) {
        const { event, timestamp } = frame;
        console.group('Handle event: ' + event.kind() + ' at timestamp: ' + new Date(Number(timestamp)).toLocaleString());
        try {
          this.#gameContext.prepareForNextEvent(timestamp);
          try {
            let context = new GameContext(this.#gameContext);
            await this.#handler.handleEvent(context, event);
            this.#gameContext = context;
          } catch (err: any) {
            console.error(err);
          }
          await this.invokeEventCallback(event);
        } catch (e: any) {
          console.log("Game context in error:", this.#gameContext);
          throw e;
        } finally {
          console.groupEnd();
        }
      } else if (frame === 'disconnected') {
        if (this.#onConnectionState !== undefined) {
          this.#onConnectionState('disconnected')
        }
        console.group('Disconnected, try reset state and context');
        try {
          const gameAccount = await this.__getGameAccount();
          this.#gameContext = new GameContext(gameAccount);
          this.#gameContext.applyCheckpoint(gameAccount.checkpointAccessVersion, this.#gameContext.settleVersion);
          await this.#connection.connect(new SubscribeEventParams({ settleVersion: this.#gameContext.settleVersion }));
          await this.__initializeState(gameAccount);
        } finally {
          console.groupEnd();
        }
      } else if (frame === 'connected') {
        if (this.#onConnectionState !== undefined) {
          this.#onConnectionState('connected')
        }
      } else if (frame === 'closed') {
        if (this.#onConnectionState !== undefined) {
          this.#onConnectionState('closed')
        }
      } else if (frame === 'reconnected') {
        if (this.#onConnectionState !== undefined) {
          this.#onConnectionState('reconnected')
        }
      } else {
        console.log('Subscribe stream ended')
        break;
      }
    }
  }

  /**
   * Join game.
   */
  async join(params: JoinOpts): Promise<TransactionResult<void>> {
    const gameAccount = await this.#transport.getGameAccount(this.gameAddr);
    if (gameAccount === undefined) {
      throw new Error('Game account not found');
    }
    const playersCount = gameAccount.players.length;
    if (gameAccount.maxPlayers <= playersCount) {
      throw new Error('Game is full');
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
      throw new Error('Game is full');
    }

    const publicKey = await this.#encryptor.exportPublicKey();

    return await this.#transport.join(this.#wallet, {
      gameAddr: this.gameAddr,
      amount: params.amount,
      accessVersion: gameAccount.accessVersion,
      position,
      verifyKey: publicKey.ec,
    });
  }

  /**
   * Submit an event.
   */
  submitEvent(raw: Uint8Array): Promise<void>;
  submitEvent(customEvent: ICustomEvent): Promise<void>;
  async submitEvent(arg: ICustomEvent | Uint8Array): Promise<void> {
    let raw = arg instanceof Uint8Array ? arg : arg.serialize();
    const event = new Custom({ sender: this.playerAddr, raw });
    const connState = await this.#connection.submitEvent(
      new SubmitEventParams({
        event,
      })
    );
    if (connState !== undefined && this.#onConnectionState !== undefined) {
      this.#onConnectionState(connState);
    }
  }

  /**
   * Submit a message, contains arbitrary content.
   */
  async submitMessage(message: string) {
    const connState = await this.#connection.submitMessage(
      new SubmitMessageParams({
        content: message,
      })
    );
    if (connState !== undefined && this.#onConnectionState !== undefined) {
      this.#onConnectionState(connState);
    }
  }

  /**
   * Get hidden knowledge by random id. The result contains both
   * public and private information.  For performance reason, it's
   * better to cache the result somewhere instead of calling this
   * function frequently.
   */
  async getRevealed(randomId: number): Promise<Map<number, string>> {
    return this.#decryptionCache.get(randomId) || new Map();
  }

  /**
   * Close current event subscription.
   */
  async close() {}

  /**
   * Exit current game.
   */
  async exit(): Promise<void>;
  async exit(keepConnection: boolean): Promise<void>;
  async exit(keepConnection: boolean = false) {
    await this.#connection.exitGame({ keepConnection });
  }

  get info(): GameInfo {
    return this.#info;
  }
}
