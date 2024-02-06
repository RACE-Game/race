import {
  BroadcastFrameEvent,
  BroadcastFrameMessage,
  BroadcastFrameTxState,
  BroadcastFrameSync,
  ConnectionState,
  IConnection,
  SubmitEventParams,
  SubscribeEventParams,
  ConnectionSubscription,
  BroadcastFrame,
  SubmitMessageParams,
} from './connection';
import { GameContext } from './game-context';
import { GameContextSnapshot } from './game-context-snapshot';
import { ITransport } from './transport';
import { IWallet } from './wallet';
import { Handler, InitAccount } from './handler';
import { IEncryptor } from './encryptor';
import { GameAccount } from './accounts';
import { PlayerConfirming } from './tx-state';
import { Client } from './client';
import { Custom, GameEvent, ICustomEvent, Join } from './events';
import { DecryptionCache } from './decryption-cache';
import { ConnectionStateCallbackFunction, EventCallbackFunction, GameInfo, LoadProfileCallbackFunction, MessageCallbackFunction, TxStateCallbackFunction } from './types';

const MAX_RETRIES = 3;

export type BaseClientCtorOpts = {
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
  onLoadProfile: LoadProfileCallbackFunction;
  encryptor: IEncryptor;
  info: GameInfo;
  decryptionCache: DecryptionCache;
};

export class BaseClient {
  __gameAddr: string;
  __handler: Handler;
  __wallet: IWallet;
  __client: Client;
  __transport: ITransport;
  __connection: IConnection;
  __gameContext: GameContext;
  __onEvent: EventCallbackFunction;
  __onMessage?: MessageCallbackFunction;
  __onTxState?: TxStateCallbackFunction;
  __onConnectionState?: ConnectionStateCallbackFunction;
  __onLoadProfile: LoadProfileCallbackFunction;
  __encryptor: IEncryptor;
  __info: GameInfo;
  __decryptionCache: DecryptionCache;

  constructor(opts: BaseClientCtorOpts) {
    this.__gameAddr = opts.gameAddr;
    this.__handler = opts.handler;
    this.__wallet = opts.wallet;
    this.__client = opts.client;
    this.__transport = opts.transport;
    this.__connection = opts.connection;
    this.__gameContext = opts.gameContext;
    this.__onEvent = opts.onEvent;
    this.__onMessage = opts.onMessage;
    this.__onTxState = opts.onTxState;
    this.__onConnectionState = opts.onConnectionState;
    this.__encryptor = opts.encryptor;
    this.__info = opts.info;
    this.__decryptionCache = opts.decryptionCache;
    this.__onLoadProfile = opts.onLoadProfile;
  }

  get playerAddr(): string {
    return this.__wallet.walletAddr;
  }

  get playerId(): bigint | undefined {
    return this.__gameContext.addrToId(this.__wallet.walletAddr);
  }

  get gameAddr(): string {
    return this.__gameAddr;
  }

  get gameContext(): GameContext {
    return this.__gameContext;
  }

  get info(): GameInfo {
    return this.__info;
  }

  /**
   * Get hidden knowledge by random id. The result contains both
   * public and private information.  For performance reason, it's
   * better to cache the result somewhere instead of calling this
   * function frequently.
   */
  async getRevealed(randomId: number): Promise<Map<number, string>> {
    return this.__decryptionCache.get(randomId) || new Map();
  }

  /**
   * Exit current game.
   */
  async exit(): Promise<void>;
  async exit(keepConnection: boolean): Promise<void>;
  async exit(keepConnection: boolean = false) {
    await this.__connection.exitGame({ keepConnection });
  }

  /**
   * Detach the game connection.
   */
  detach() {
    this.__connection.disconnect();
  }

  /**
   * Parse the id to player's address.
   *
   * Throw an error when it fails.
   */
  idToAddr(id: bigint): string {
    return this.__gameContext.idToAddr(id);
  }

  /**
   * Parse the player's address to its id.
   *
   * Throw an error when it fails.
   */
  addrToId(addr: string): bigint {
    return this.__gameContext.addrToId(addr);
  }

  /**
   * Submit an event.
   */
  submitEvent(raw: Uint8Array): Promise<void>;
  submitEvent(customEvent: ICustomEvent): Promise<void>;
  async submitEvent(arg: ICustomEvent | Uint8Array): Promise<void> {
    let raw = arg instanceof Uint8Array ? arg : arg.serialize();
    const id = this.__gameContext.addrToId(this.playerAddr);
    const event = new Custom({ sender: id, raw });
    const connState = await this.__connection.submitEvent(
      new SubmitEventParams({
        event,
      })
    );
    if (connState !== undefined && this.__onConnectionState !== undefined) {
      this.__onConnectionState(connState);
    }
  }

  /**
   * Submit a message.
   */
  async submitMessage(content: string): Promise<void> {
    const connState = await this.__connection.submitMessage(new SubmitMessageParams({
      content
    }));
    if (connState !== undefined && this.__onConnectionState !== undefined) {
      this.__onConnectionState(connState);
    }
  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    console.groupCollapsed('Attach to game');
    let sub;
    try {
      await this.__client.attachGame();
      sub = this.__connection.subscribeEvents();
      const gameAccount = await this.__getGameAccount();
      const initAccount = InitAccount.createFromGameAccount(gameAccount, this.gameContext.accessVersion, this.gameContext.settleVersion);
      this.__gameContext = new GameContext(gameAccount);
      this.__gameContext.applyCheckpoint(gameAccount.checkpointAccessVersion, this.__gameContext.settleVersion);

      for (const p of gameAccount.players) this.__onLoadProfile(p.accessVersion, p.addr);
      await this.__connection.connect(new SubscribeEventParams({ settleVersion: this.__gameContext.settleVersion }));
      await this.__initializeState(initAccount);
    } catch (e) {
      console.error('Attaching game failed', e);
      throw e;
    } finally {
      console.groupEnd();
    }
    await this.__processSubscription(sub);
  }

  async __invokeEventCallback(event: GameEvent | undefined, isHistory: boolean) {
    const snapshot = new GameContextSnapshot(this.__gameContext);
    const state = this.__gameContext.handlerState;
    this.__onEvent(snapshot, state, event, isHistory);
  }

  async __initializeState(initAccount: InitAccount): Promise<void> {
    console.log('Initialize state with', initAccount);
    await this.__handler.initState(this.__gameContext, initAccount);
    console.log('State initialized');
    await this.__invokeEventCallback(undefined, true);
  }

  async __getGameAccount(): Promise<GameAccount> {
    let retries = 0;
    while (true) {
      if (retries === MAX_RETRIES) {
          throw new Error(`Game account not found, after ${retries} retries`);
      }
      try {
        const gameAccount = await this.__transport.getGameAccount(this.gameAddr);
        if (gameAccount === undefined) {
          retries += 1;
          continue;
        }
        return gameAccount;
      } catch (e: any) {
        console.warn(e, 'Failed to fetch game account, will retry in 3s');
        await new Promise(r => setTimeout(r, 3000));
        retries += 1;
        continue;
      }
    }
  }

  async __processSubscription(sub: ConnectionSubscription) {
    for await (const item of sub) {
      if (item === undefined) {
        console.log('Subscribe stream ended')
        break;
      } else if (item instanceof BroadcastFrame) {
        await this.__handleBroadcastFrame(item);
      } else {
        await this.__handleConnectionState(item);
      }
    }
  }

  async __handleBroadcastFrame(frame: BroadcastFrame) {
    if (frame instanceof BroadcastFrameMessage) {
      console.groupCollapsed('Receive message broadcast');
      try {
        if (this.__onMessage !== undefined) {
          const { message } = frame;
          this.__onMessage(message);
        }
      } finally {
        console.groupEnd();
      }
    } else if (frame instanceof BroadcastFrameTxState) {
      console.groupCollapsed('Receive transaction state broadcast');
      try {
        if (this.__onTxState !== undefined) {
          const { txState } = frame;
          if (txState instanceof PlayerConfirming) {
            txState.confirmPlayers.forEach(p => {
              this.__onLoadProfile(p.id, p.addr);
            });
          }
          this.__onTxState(txState);
        }
      } finally {
        console.groupEnd();
      }
    } else if (frame instanceof BroadcastFrameSync) {
      console.groupCollapsed('Receive sync broadcast');
      try {
        for (const node of frame.newServers) {
          this.__gameContext.addNode(node.addr, node.accessVersion,
            node.addr === frame.transactor_addr ? 'transactor' : 'validator');
        }
        for (const node of frame.newPlayers) {
          this.__gameContext.addNode(node.addr, node.accessVersion, 'player');
          this.__onLoadProfile(node.accessVersion, node.addr);
        }
        this.__gameContext.setAccessVersion(frame.accessVersion);
      } finally {
        console.groupEnd();
      }
    } else if (frame instanceof BroadcastFrameEvent) {
      const { event, timestamp } = frame;
      console.groupCollapsed('Handle event: ' + event.kind() + ' at timestamp: ' + new Date(Number(timestamp)).toLocaleString());
      console.log('Event: ', event);
      try {
        this.__gameContext.prepareForNextEvent(timestamp);
        try {
          let context = new GameContext(this.__gameContext);
          await this.__handler.handleEvent(context, event);
          this.__gameContext = context;
          console.log("Game Context:", this.__gameContext);
        } catch (err: any) {
          console.error(err);
        }
        await this.__invokeEventCallback(event, frame.remain !== 0);
      } catch (e: any) {
        console.log("Game context in error:", this.__gameContext);
        throw e;
      } finally {
        console.groupEnd();
      }
    }
  }

  async __handleConnectionState(state: ConnectionState) {
    if (state === 'disconnected') {
      if (this.__onConnectionState !== undefined) {
        this.__onConnectionState('disconnected')
      }
      console.log('Disconnected, try reset state and context');
      const gameAccount = await this.__getGameAccount();
      this.__gameContext = new GameContext(gameAccount);
      const initAccount = InitAccount.createFromGameAccount(gameAccount, this.__gameContext.accessVersion, this.__gameContext.settleVersion);
      this.__gameContext.applyCheckpoint(gameAccount.checkpointAccessVersion, this.__gameContext.settleVersion);
      await this.__connection.connect(new SubscribeEventParams({ settleVersion: this.__gameContext.settleVersion }));
      await this.__initializeState(initAccount);
    } else if (state === 'connected') {
      if (this.__onConnectionState !== undefined) {
        this.__onConnectionState('connected')
      }
    } else if (state === 'closed') {
      if (this.__onConnectionState !== undefined) {
        this.__onConnectionState('closed')
      }
    } else if (state === 'reconnected') {
      if (this.__onConnectionState !== undefined) {
        this.__onConnectionState('reconnected')
      }
    }
  }
}
