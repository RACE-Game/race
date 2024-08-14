import { BaseClient } from './base-client';
import { Client } from './client';
import { IConnection, SubscribeEventParams } from './connection';
import { DecryptionCache } from './decryption-cache';
import { IEncryptor } from './encryptor';
import { GameContext } from './game-context';
import { Handler } from './handler';
import { ITransport } from './transport';
import { GameInfo, ConnectionStateCallbackFunction, EventCallbackFunction, MessageCallbackFunction, TxStateCallbackFunction, ErrorCallbackFunction } from './types';
import { IWallet } from './wallet';
import { Init } from './events';

export type SubClientCtorOpts = {
  gameAddr: string;
  gameId: number;
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
}

export class SubClient extends BaseClient {

  __gameId: number;

  constructor(opts: SubClientCtorOpts) {
    super({
      onLoadProfile: (_id: bigint, _addr: string) => {},
      logPrefix: `SubGame#${opts.gameId}|`,
      ...opts
    })
    this.__gameId = opts.gameId;
  }

  get gameId(): number {
    return this.__gameId;
  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    console.group(`${this.__logPrefix}Attach to game`);
    let sub;
    try {
      console.log('Checkpoint:', this.__gameContext.checkpoint);
      await this.__attachGameWithRetry();
      sub = this.__connection.subscribeEvents();
      console.log('Subscription:', sub);
      const settleVersion = this.__gameContext.checkpointVersion();
      await this.__connection.connect(new SubscribeEventParams({ settleVersion }));
      const initAccount = this.__gameContext.initAccount();
      await this.__handler.initState(this.__gameContext, initAccount);
      this.__checkStateSha(this.__gameContext.checkpointStateSha, 'checkpoint-state-sha-mismatch');
      this.__invokeEventCallback(new Init());
    } catch (e) {
      console.error('Attaching game failed', e);
      throw e;
    } finally {
      console.groupEnd();
    }
    await this.__processSubscription(sub);
  }
}
