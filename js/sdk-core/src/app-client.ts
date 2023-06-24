import { BroadcastFrameEvent, BroadcastFrameInit, Connection, IConnection, SubmitEventParams, SubscribeEventParams } from './connection';
import { GameContext } from './game-context';
import { GameContextSnapshot } from './game-context-snapshot';
import { ITransport } from './transport';
import { IWallet } from './wallet';
import { Handler, InitAccount } from './handler';
import { Encryptor, IEncryptor } from './encryptor';
import { SdkError } from './error';
import { GameAccount, PlayerProfile } from './accounts';
import { Client } from './client';
import { Custom, GameEvent, ICustomEvent } from './events';
import { ProfileCache } from './profile-cache';

export type EventCallbackFunction = (context: GameContextSnapshot, state: Uint8Array, event: GameEvent | undefined) => void;

export type AppClientInitOpts = {
  transport: ITransport,
  wallet: IWallet,
  gameAddr: string,
  callback: EventCallbackFunction,
};

export type JoinOpts = {
  amount: bigint,
  position?: number,
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
  #callback: EventCallbackFunction;
  #encryptor: IEncryptor;
  #profileCaches: ProfileCache;

  constructor(
    gameAddr: string,
    handler: Handler,
    wallet: IWallet,
    client: Client,
    transport: ITransport,
    connection: IConnection,
    gameContext: GameContext,
    initGameAccount: GameAccount,
    callback: EventCallbackFunction,
    encryptor: IEncryptor,
  ) {
    this.#gameAddr = gameAddr;
    this.#handler = handler;
    this.#wallet = wallet;
    this.#client = client;
    this.#transport = transport;
    this.#connection = connection;
    this.#gameContext = gameContext;
    this.#initGameAccount = initGameAccount;
    this.#callback = callback;
    this.#encryptor = encryptor;
    this.#profileCaches = new ProfileCache(transport);
  }

  static async initialize(
    opts: AppClientInitOpts
  ): Promise<AppClient> {
    const { transport, wallet, gameAddr, callback } = opts;
    console.group("AppClient initialization");
    try {
      const encryptor = await Encryptor.create();
      const playerAddr = wallet.walletAddr;
      const gameAccount = await transport.getGameAccount(gameAddr);
      if (gameAccount === undefined) {
        throw SdkError.gameAccountNotFound(gameAddr);
      }
      console.log("Game account:", gameAccount);
      const gameBundle = await transport.getGameBundle(gameAccount.bundleAddr);
      if (gameBundle === undefined) {
        throw SdkError.gameBundleNotFound(gameAccount.bundleAddr);
      }
      console.log("Game bundle:", gameBundle);
      const transactorAddr = gameAccount.transactorAddr;
      if (transactorAddr === undefined) {
        throw SdkError.gameNotServed(gameAddr);
      }
      console.log("Transactor address:", transactorAddr);
      const transactorAccount = await transport.getServerAccount(transactorAddr);
      if (transactorAccount === undefined) {
        throw SdkError.transactorAccountNotFound(transactorAddr);
      }
      const handler = await Handler.initialize(gameBundle, encryptor);
      const connection = Connection.initialize(playerAddr, transactorAccount.endpoint, encryptor);
      const client = new Client(playerAddr, gameAddr, transport, encryptor, connection);
      const gameContext = new GameContext(gameAccount);
      return new AppClient(gameAddr, handler, wallet, client, transport, connection, gameContext, gameAccount, callback, encryptor);
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

  async invokeCallback(event: GameEvent | undefined) {
    const snapshot = new GameContextSnapshot(this.#gameContext);
    await this.#profileCaches.injectProfiles(snapshot);
    const state = this.#gameContext.handlerState;
    this.#callback(snapshot, state, event);
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
        console.group('Initialize handler state')
        try {
          const { accessVersion, settleVersion } = frame;
          console.log('Access version:', accessVersion);
          console.log('Settle version:', settleVersion);
          this.#gameContext.applyCheckpoint(accessVersion, settleVersion);
          const initAccount = InitAccount.createFromGameAccount(this.#initGameAccount, accessVersion, settleVersion);
          console.log('Init account:', initAccount);
          await this.#handler.initState(this.#gameContext, initAccount);
          await this.invokeCallback(undefined);
        } finally {
          console.groupEnd();
        }
      } else if (frame instanceof BroadcastFrameEvent) {
        const { event, timestamp } = frame;
        console.group('Handle event: ' + event.kind());
        try {
          console.log('Event:', event);
          console.log('Timestamp:', new Date(Number(timestamp)).toLocaleTimeString());
          this.#gameContext.timestamp = timestamp;
          try {
            let context = new GameContext(this.#gameContext);
            await this.#handler.handleEvent(context, event);
            this.#gameContext = context;
            console.log('Game context:', this.#gameContext);
          } catch (err) {
            console.warn(err);
          }
          await this.invokeCallback(event);
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
    await this.#connection.submitEvent(this.#gameAddr, new SubmitEventParams({
      event
    }));
  }

  /**
   * Get hidden knowledge by random id. The result contains both
   * public and private information.  For performance reason, it's
   * better to cache the result somewhere instead of calling this
   * function frequently.
   */
  async getRevealed(randomId: number): Promise<Map<number, string>> {
    return await this.#client.decrypt(this.#gameContext, randomId);
  }

  /**
   * Close current event subscription.
   */
  async close() {
  }

  /**
   * Exit current game.
   */
  async exit() {
    await this.#connection.exitGame(this.#gameAddr, {});
  }
}
