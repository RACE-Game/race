import { BroadcastFrameEvent, BroadcastFrameInit, Connection, IConnection, SubscribeEventParams } from './connection';
import { GameContext } from './game-context';
import { ITransport } from './transport';
import { IWallet } from './wallet';
import { Handler, InitAccount } from './handler';
import { Encryptor } from './encryptor';
import { SdkError } from './error';
import { GameAccount, PlayerProfile } from './accounts';
import { Client } from './client';
import { Custom, ICustomEvent } from './events';

export type EventCallbackFunction = (context: GameContext, state: Uint8Array, event: Event | undefined) => void;

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

  constructor(
    gameAddr: string,
    handler: Handler,
    wallet: IWallet,
    client: Client,
    transport: ITransport,
    connection: IConnection,
    gameContext: GameContext,
    initGameAccount: GameAccount,
    callback: EventCallbackFunction
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
  }

  static async initialize(
    transport: ITransport,
    wallet: IWallet,
    gameAddr: string,
    callback: EventCallbackFunction
  ): Promise<AppClient> {
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
    return new AppClient(gameAddr, handler, wallet, client, transport, connection, gameContext, gameAccount, callback);
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

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    await this.#client.attachGame();
    const settleVersion = this.#gameContext.settleVersion;
    let sub = this.#connection.subscribeEvents(this.#gameAddr, new SubscribeEventParams({ settleVersion }));
    for await (const frame of sub) {
      if (frame instanceof BroadcastFrameInit) {
        this.#gameContext.applyCheckpoint(frame.accessVersion, frame.settleVersion);
        const initAccount = InitAccount.createFromGameAccount(this.#initGameAccount, frame.accessVersion, frame.settleVersion);

        this.#handler.initState(this.#gameContext, initAccount);

        console.log("init called");
      } else if (frame instanceof BroadcastFrameEvent) {

      } else {
        throw new Error('Invalid broadcast frame');
      }
    }
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
