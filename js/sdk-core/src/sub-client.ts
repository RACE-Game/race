import { BaseClient, ConnectionStateCallbackFunction, EventCallbackFunction, GameInfo, MessageCallbackFunction, TxStateCallbackFunction } from './base-client';
import { Client } from './client';
import { IConnection } from './connection';
import { DecryptionCache } from './decryption-cache';
import { IEncryptor } from './encryptor';
import { GameContext } from './game-context';
import { Handler } from './handler';
import { ITransport } from './transport';
import { IWallet } from './wallet';

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
  encryptor: IEncryptor;
  info: GameInfo;
  decryptionCache: DecryptionCache;
}

export class SubClient extends BaseClient {

  #subId: number;

  constructor(opts: SubClientCtorOpts) {
    super({
      onLoadProfile: (_: string) => {},
      ...opts
    })
    this.#subId = opts.subId;
  }

  get subId(): number {
    return this.#subId;
  }
}
