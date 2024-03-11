import { BaseClient } from './base-client';
import { Client } from './client';
import { IConnection, SubscribeEventParams } from './connection';
import { DecryptionCache } from './decryption-cache';
import { IEncryptor } from './encryptor';
import { GameContext } from './game-context';
import { InitAccount } from './init-account';
import { Handler } from './handler';
import { ITransport } from './transport';
import { GameInfo, ConnectionStateCallbackFunction, EventCallbackFunction, MessageCallbackFunction, TxStateCallbackFunction, ErrorCallbackFunction } from './types';
import { IWallet } from './wallet';
import { Init } from './events';

export type SubClientCtorOpts = {
  gameAddr: string;
  subId: number;
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
  initAccount: InitAccount;
}

export class SubClient extends BaseClient {

  __subId: number;
  __initAccount: InitAccount;

  constructor(opts: SubClientCtorOpts) {
    super({
      onLoadProfile: (_id: bigint, _addr: string) => {},
      logPrefix: `SubGame#${opts.subId}|`,
      ...opts
    })
    this.__subId = opts.subId;
    this.__initAccount = opts.initAccount;
  }

  get subId(): number {
    return this.__subId;
  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    console.groupCollapsed(`${this.__logPrefix}Attach to game`);
    let sub;
    try {
      await this.__client.attachGame();
      sub = this.__connection.subscribeEvents();
      await this.__connection.connect(new SubscribeEventParams({ settleVersion: this.__gameContext.settleVersion }));
      await this.__handler.initState(this.__gameContext, this.__initAccount);
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
