import {
  BroadcastFrameEvent,
  BroadcastFrameInit,
  BroadcastFrameMessage,
  Connection,
  IConnection,
  Message,
  SubmitEventParams,
  SubmitMessageParams,
  SubscribeEventParams,
} from './connection';
import { GameContext } from './game-context';
import { GameContextSnapshot } from './game-context-snapshot';
import { ITransport } from './transport';
import { IWallet } from './wallet';
import { Handler, InitAccount } from './handler';
import { Encryptor, IEncryptor } from './encryptor';
import { SdkError } from './error';
import { GameAccount, IToken, PlayerProfile } from './accounts';
import { Client } from './client';
import { Custom, GameEvent, ICustomEvent } from './events';
import { ProfileCache } from './profile-cache';
import { IStorage } from './storage';
import { DecryptionCache } from './decryption-cache';

export type EventCallbackFunction = (
  context: GameContextSnapshot,
  state: Uint8Array,
  event: GameEvent | undefined
) => void;
export type MessageCallbackFunction = (message: Message) => void;

export type AppClientInitOpts = {
  transport: ITransport;
  wallet: IWallet;
  gameAddr: string;
  onEvent: EventCallbackFunction;
  onMessage: MessageCallbackFunction;
  storage?: IStorage;
};

export type JoinOpts = {
  amount: bigint;
  position?: number;
};

export type GameInfo = {
  title: string;
  maxPlayers: number;
  minDeposit: bigint;
  maxDeposit: bigint;
  token: IToken;
};

export class AppClient {
  #gameAddr: string;
  #handler: Handler;
  #wallet: IWallet;
  #client: Client;
  #transport: ITransport;
  #connection: IConnection;
  #gameContext: GameContext;
  #initGameAccount: GameAccount;
  #onEvent: EventCallbackFunction;
  #onMessage: MessageCallbackFunction;
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
    initGameAccount: GameAccount,
    onEvent: EventCallbackFunction,
    onMessage: MessageCallbackFunction,
    encryptor: IEncryptor,
    info: GameInfo,
    decryptionCache: DecryptionCache
  ) {
    this.#gameAddr = gameAddr;
    this.#handler = handler;
    this.#wallet = wallet;
    this.#client = client;
    this.#transport = transport;
    this.#connection = connection;
    this.#gameContext = gameContext;
    this.#initGameAccount = initGameAccount;
    this.#onEvent = onEvent;
    this.#onMessage = onMessage;
    this.#encryptor = encryptor;
    this.#profileCaches = new ProfileCache(transport);
    this.#info = info;
    this.#decryptionCache = decryptionCache;
  }

  static async initialize(opts: AppClientInitOpts): Promise<AppClient> {
    const { transport, wallet, gameAddr, onEvent, onMessage, storage } = opts;
    console.group('AppClient initialization');
    try {
      const playerAddr = wallet.walletAddr;
      const encryptor = await Encryptor.create(playerAddr, storage);
      const gameAccount = await transport.getGameAccount(gameAddr);
      if (gameAccount === undefined) {
        throw SdkError.gameAccountNotFound(gameAddr);
      }
      console.log('Game account:', gameAccount);
      const gameBundle = await transport.getGameBundle(gameAccount.bundleAddr);
      if (gameBundle === undefined) {
        throw SdkError.gameBundleNotFound(gameAccount.bundleAddr);
      }
      console.log('Game bundle:', gameBundle);
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
      const connection = Connection.initialize(playerAddr, transactorAccount.endpoint, encryptor);
      const client = new Client(playerAddr, gameAddr, transport, encryptor, connection);
      const handler = await Handler.initialize(gameBundle, encryptor, client, decryptionCache);
      const gameContext = new GameContext(gameAccount);
      const token = await transport.getToken(gameAccount.tokenAddr);
      if (token === undefined) {
        throw SdkError.tokenNotFound(gameAccount.tokenAddr);
      }
      const info = {
        title: gameAccount.title,
        minDeposit: gameAccount.minDeposit,
        maxDeposit: gameAccount.maxDeposit,
        maxPlayers: gameAccount.maxPlayers,
        token,
      };
      return new AppClient(
        gameAddr,
        handler,
        wallet,
        client,
        transport,
        connection,
        gameContext,
        gameAccount,
        onEvent,
        onMessage,
        encryptor,
        info,
        decryptionCache
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

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    await this.#client.attachGame();
    const settleVersion = this.#gameContext.settleVersion;
    let sub = this.#connection.subscribeEvents(this.#gameAddr, new SubscribeEventParams({ settleVersion }));
    for await (const frame of sub) {
      if (frame instanceof BroadcastFrameInit) {
        console.group('Initialize handler state');
        try {
          const { checkpointState, accessVersion, settleVersion } = frame;
          console.log('Access version:', accessVersion);
          console.log('Settle version:', settleVersion);
          this.#gameContext.applyCheckpoint(accessVersion, settleVersion);
          const initAccount = InitAccount.createFromGameAccount(this.#initGameAccount, accessVersion, settleVersion);
          console.log('Init account:', initAccount);
          await this.#handler.initState(this.#gameContext, initAccount);
          this.#gameContext.handlerState = checkpointState;
          console.log('Context created:', this.#gameContext);
          await this.invokeEventCallback(undefined);
        } finally {
          console.groupEnd();
        }
      } else if (frame instanceof BroadcastFrameMessage) {
        const { message } = frame;
        this.#onMessage(message);
      } else if (frame instanceof BroadcastFrameEvent) {
        const t0 = new Date().getTime();
        const { event, timestamp } = frame;
        console.group('Handle event: ' + event.kind());
        try {
          this.#gameContext.prepareForNextEvent(timestamp);
          try {
            let context = new GameContext(this.#gameContext);
            await this.#handler.handleEvent(context, event);
            this.#gameContext = context;
          } catch (err: any) {
            console.error(err);
          }
          console.log("Cost: ", new Date().getTime() - t0, "ms");
          await this.invokeEventCallback(event);
        } catch (e: any) {
          console.log("Game context in error:", this.#gameContext);
          throw e;
        } finally {
          console.groupEnd();
        }
      } else {
        break;
      }
    }
  }

  /**
   * Join game.
   */
  async join(params: JoinOpts) {
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

    this.#transport.join(this.#wallet, {
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
    await this.#connection.submitEvent(
      this.#gameAddr,
      new SubmitEventParams({
        event,
      })
    );
  }

  /**
   * Submit a message, contains arbitrary content.
   */
  async submitMessage(message: string) {
    await this.#connection.submitMessage(
      this.#gameAddr,
      new SubmitMessageParams({
        content: message,
      })
    );
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
  async exit() {
    await this.#connection.exitGame(this.#gameAddr, {});
  }

  get info(): GameInfo {
    return this.#info;
  }
}
