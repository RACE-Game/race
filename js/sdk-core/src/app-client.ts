import { BroadcastFrameEvent, BroadcastFrameInit, Connection, IConnection, SubscribeEventParams } from './connection';
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

export type EventCallbackFunction = (context: GameContextSnapshot, state: Uint8Array, event: GameEvent | undefined) => void;

export type AppClientInitOpts = {
  transport: ITransport,
  wallet: IWallet,
  gameAddr: string,
  callback: EventCallbackFunction,
};

export type JoinOpts = {
  amount: bigint,
  position: number,
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
  }

  static async initialize(
    opts: AppClientInitOpts
  ): Promise<AppClient> {
    const { transport, wallet, gameAddr, callback } = opts;

    const encryptor = await Encryptor.create();
    const playerAddr = wallet.walletAddr;
    const gameAccount = await transport.getGameAccount(gameAddr);
    if (gameAccount === undefined) {
      throw SdkError.gameAccountNotFound(gameAddr);
    }
    const gameBundle = await transport.getGameBundle(gameAccount.bundleAddr);
    if (gameBundle === undefined) {
      throw SdkError.gameBundleNotFound(gameAccount.bundleAddr);
    }
    const transactorAddr = gameAccount.transactorAddr;
    if (transactorAddr === undefined) {
      throw SdkError.gameNotServed(gameAddr);
    }
    const transactorAccount = await transport.getServerAccount(transactorAddr);
    if (transactorAccount === undefined) {
      throw SdkError.transactorAccountNotFound(transactorAddr);
    }
    const handler = await Handler.initialize(gameBundle, encryptor);
    const connection = Connection.initialize(playerAddr, transactorAccount.endpoint, encryptor);
    const client = new Client(playerAddr, gameAddr, transport, encryptor, connection);
    const gameContext = new GameContext(gameAccount);
    return new AppClient(gameAddr, handler, wallet, client, transport, connection, gameContext, gameAccount, callback, encryptor);
  }

  get playerAddr() {
    return this.#wallet.walletAddr;
  }

  get gameAddr() {
    return this.#gameAddr;
  }

  /**
   * Get player profile by its wallet address.
   */
  async getProfile(addr: string): Promise<PlayerProfile | undefined> {
    return await this.#transport.getPlayerProfile(addr);
  }

  invokeCallback(event: GameEvent | undefined) {
    const snapshot = new GameContextSnapshot(this.#gameContext);
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
      console.log("Await frame:", frame);
      if (frame instanceof BroadcastFrameInit) {
        const { accessVersion, settleVersion } = frame;
        this.#gameContext.applyCheckpoint(accessVersion, settleVersion);
        const initAccount = InitAccount.createFromGameAccount(this.#initGameAccount, accessVersion, settleVersion);
        this.#handler.initState(this.#gameContext, initAccount);
        this.invokeCallback(undefined);
      } else if (frame instanceof BroadcastFrameEvent) {
        const { event, timestamp } = frame;
        this.#gameContext.timestamp = timestamp;
        this.#handler.handleEvent(this.#gameContext, event);
        this.invokeCallback(event);
      } else {
        throw new Error('Invalid broadcast frame');
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
    let position: number | undefined = undefined;
    for (let i = 0; i < gameAccount.maxPlayers; i++) {
      if (gameAccount.players.find(p => p.position === i) === undefined) {
        position = i;
        break;
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
  async submitEvent(customEvent: ICustomEvent): Promise<void> {
    const raw = customEvent.serialize();
    const event = new Custom({ sender: this.playerAddr, raw });
    await this.#connection.submitEvent(this.#gameAddr, { event });
  }

  getRevealed(randomId: bigint): Map<number, string> {
    // this.#client.decrypt(context, randomId);
    return new Map();
  }

  /**
   * Close current event subscription.
   */
  async close() { }

  /**
   * Exit current game.
   */
  async exit() {
    await this.#connection.exitGame(this.#gameAddr, {});
  }
}
