import {
  ConnectionState,
  IConnection,
  SubmitEventParams,
  SubscribeEventParams,
  ConnectionSubscription,
  SubmitMessageParams,
} from './connection';
import { EventEffects, GameContext } from './game-context';
import { GameContextSnapshot } from './game-context-snapshot';
import { ITransport } from './transport';
import { IWallet } from './wallet';
import { Handler } from './handler';
import { InitAccount } from './init-account';
import { IEncryptor, sha256 } from './encryptor';
import { GameAccount } from './accounts';
import { PlayerConfirming } from './tx-state';
import { Client } from './client';
import { CheckpointReady, Custom, EndOfHistory, GameEvent, ICustomEvent, Init } from './events';
import { DecryptionCache } from './decryption-cache';
import { ConnectionStateCallbackFunction, ErrorCallbackFunction, ErrorKind, EventCallbackFunction, GameInfo, LoadProfileCallbackFunction, MessageCallbackFunction, TxStateCallbackFunction } from './types';
import { BroadcastFrame, BroadcastFrameEventHistories, BroadcastFrameMessage, BroadcastFrameSync, BroadcastFrameEvent, BroadcastFrameTxState } from './broadcast-frames';

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
  onError: ErrorCallbackFunction | undefined;
  onLoadProfile: LoadProfileCallbackFunction;
  encryptor: IEncryptor;
  info: GameInfo;
  decryptionCache: DecryptionCache;
  logPrefix: string;
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
  __onError?: ErrorCallbackFunction;
  __onConnectionState?: ConnectionStateCallbackFunction;
  __onLoadProfile: LoadProfileCallbackFunction;
  __encryptor: IEncryptor;
  __info: GameInfo;
  __decryptionCache: DecryptionCache;
  __logPrefix: string;

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
    this.__onError = opts.onError;
    this.__onConnectionState = opts.onConnectionState;
    this.__encryptor = opts.encryptor;
    this.__info = opts.info;
    this.__decryptionCache = opts.decryptionCache;
    this.__onLoadProfile = opts.onLoadProfile;
    this.__logPrefix = opts.logPrefix;
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

  async __attachGameWithRetry() {
    for (let i = 0; i < 10; i++) {
      const resp = await this.__client.attachGame();
      console.log('Attach response:', resp);
      if (resp === 'success') {
        break;
      } else {
        console.log(this.__logPrefix + 'Game is not ready, try again after 2 second.');
        await new Promise(r => setTimeout(r, 2000));
      }
    }

  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    console.group('Attach to game');
    let sub;
    try {
      console.log('Checkpoint:', this.__gameContext.checkpoint);
      await this.__attachGameWithRetry();
      sub = this.__connection.subscribeEvents();
      const { gameAccount } = await this.__startSubscribeAndInitState();
      for (const p of gameAccount.players) this.__onLoadProfile(p.accessVersion, p.addr);
      this.__invokeEventCallback(new Init());
    } catch (e) {
      console.error(this.__logPrefix + 'Attaching game failed', e);
      this.__invokeErrorCallback('attach-failed')
      throw e;
    } finally {
      console.groupEnd();
    }
    if (sub !== undefined) await this.__processSubscription(sub);
  }

  __invokeErrorCallback(err: ErrorKind, arg?: any) {
    if (this.__onError) {
      this.__onError(err, arg)
    } else {
      console.error(`${this.__logPrefix}An error occured: ${err}, to handle it, use \`onError\`.`)
    }
  }

  async __invokeEventCallback(event: GameEvent | undefined) {
    const snapshot = new GameContextSnapshot(this.__gameContext);
    const state = this.__gameContext.handlerState;
    this.__onEvent(snapshot, state, event);
  }

  async __getGameAccount(): Promise<GameAccount> {
    let retries = 0;
    while (true) {
      if (retries === MAX_RETRIES) {
        this.__invokeErrorCallback('onchain-data-not-found')
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
        break;
      } else if (item instanceof BroadcastFrame) {
        await this.__handleBroadcastFrame(item);
      } else {
        await this.__handleConnectionState(item);
      }
    }
  }

  async __checkStateSha(stateSha: string, err: ErrorKind) {
    const sha = await sha256(this.__gameContext.handlerState)
    if (sha !== stateSha && stateSha !== '') {
      const state = this.__gameContext.handlerState
      this.__invokeErrorCallback(err, state);
      throw new Error(`An error occurred in event loop: ${err}, game: ${this.__gameAddr}, local: ${sha}, remote: ${stateSha}`);
    } else {
      console.log('State SHA validation passed:', stateSha);
    }
  }

  async __handleEvent(event: GameEvent, timestamp: bigint, stateSha: string) {
    console.group(this.__logPrefix + 'Handle event: ' + event.kind() + ' at timestamp: ' + timestamp);
    console.log('Event: ', event);
    console.log('Game Context before:', this.__gameContext);
    let state: Uint8Array | undefined;
    let err: ErrorKind | undefined;
    let effects: EventEffects | undefined;

    try {                     // For log group
      try {
        this.__gameContext.setTimestamp(timestamp);
        effects = await this.__handler.handleEvent(this.__gameContext, event);
        state = this.__gameContext.handlerState;
      } catch (e: any) {
        console.error(this.__logPrefix, e);
        err = 'handle-event-error';
      }

      await this.__checkStateSha(stateSha, 'event-state-sha-mismatch');

      if (!err) {
        await this.__invokeEventCallback(event);
      }

      // Rebuild the game state after a checkpoint.
      if ((!err) && effects?.checkpoint) {
        const initData = this.__gameContext.initData;
        if (initData === undefined) {
          err = 'init-data-invalid'
        } else {
          const initAccount = new InitAccount({
            entryType: this.__gameContext.entryType,
            data: initData,
            players: this.__gameContext.players,
            maxPlayers: this.__gameContext.maxPlayers,
          });
          await this.__handler.initState(this.__gameContext, initAccount);
          this.__invokeEventCallback(new CheckpointReady());
          // Update state SHA and version
          const sha = await sha256(this.__gameContext.handlerState);
          this.__gameContext.checkpoint.setSha(this.__gameContext.gameId, sha);
          // this.__gameContext.checkpoint.setVersion(this.__gameContext.gameId, this.__gameContext.settleVersion);
        }
      }

      if (err) {
        this.__invokeErrorCallback(err, state);
        throw new Error(`An error occurred in event loop: ${err}`);
      }

    } finally {
      console.log('Game Context after:', this.__gameContext);
      console.groupEnd()
    }
  }

  async __handleBroadcastFrame(frame: BroadcastFrame) {
    if (frame instanceof BroadcastFrameMessage) {
      console.group(`${this.__logPrefix}Receive message broadcast`);
      try {
        if (this.__onMessage !== undefined) {
          const { message } = frame;
          console.log('Message:', message);
          this.__onMessage(message);
        }
      } finally {
        console.groupEnd();
      }
    } else if (frame instanceof BroadcastFrameTxState) {
      console.group(`${this.__logPrefix}Receive transaction state broadcast`);
      try {
        if (this.__onTxState !== undefined) {
          const { txState } = frame;
          console.log('TxState:', txState);
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
      console.group(`${this.__logPrefix}Receive sync broadcast`);
      try {
        console.log('Sync:', frame);
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
      const { event, timestamp, stateSha } = frame;
      await this.__handleEvent(event, timestamp, stateSha);
    } else if (frame instanceof BroadcastFrameEventHistories) {
      for (const h of frame.histories) {
        await this.__handleEvent(h.event, h.timestamp, h.stateSha);
      }
      this.__invokeEventCallback(new EndOfHistory())
    }
  }

  async __startSubscribeAndInitState(): Promise<{
    gameAccount: GameAccount,
    initAccount: InitAccount,
  }> {
    const gameAccount = await this.__getGameAccount();
    this.__gameContext = new GameContext(gameAccount);
    const initAccount = this.__gameContext.initAccount();
    this.__gameContext.applyCheckpoint(gameAccount.checkpoint.accessVersion, this.__gameContext.settleVersion);
    await this.__connection.connect(new SubscribeEventParams({ settleVersion: this.__gameContext.checkpointVersion() }));
    await this.__handler.initState(this.__gameContext, initAccount);
    this.__checkStateSha(this.__gameContext.checkpointStateSha, 'checkpoint-state-sha-mismatch');
    return { gameAccount, initAccount };
  }

  async __handleConnectionState(state: ConnectionState) {
    if (state === 'disconnected') {
      if (this.__onConnectionState !== undefined) {
        this.__onConnectionState('disconnected')
      }
      console.log('Disconnected, try reset state and context');
      await this.__startSubscribeAndInitState();
      this.__invokeEventCallback(new Init());
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
