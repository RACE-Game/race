import { deserialize, serialize } from '@race/borsh';
import { GameAccount, GameBundle, PlayerJoin, ServerJoin } from './accounts';
import { GameEvent } from './events';
import { GameContext } from './game-context';
import { IEncryptor } from './encryptor';
import { Effect } from './effect';

/**
 * A subset of GameAccount, used in handler initialization.
 */
export interface IInitAccount {
  addr: string;
  players: PlayerJoin[];
  servers: ServerJoin[];
  data: Uint8Array;
  accessVersion: bigint;
  settleVersion: bigint;
}

export class InitAccount {
  readonly addr: string;
  readonly players: PlayerJoin[];
  readonly servers: ServerJoin[];
  readonly data: Uint8Array;
  readonly accessVersion: bigint;
  readonly settleVersion: bigint;
  constructor(fields: IInitAccount) {
    this.addr = fields.addr;
    this.accessVersion = fields.accessVersion;
    this.settleVersion = fields.settleVersion;
    this.data = fields.data;
    this.players = fields.players;
    this.servers = fields.servers;
  }
  static createFromGameAccount(
    gameAccount: GameAccount,
    transactorAccessVersion: bigint,
    transactorSettleVersion: bigint
  ): InitAccount {
    let { addr, players, servers, data } = gameAccount;
    players = players.filter(p => p.accessVersion <= transactorAccessVersion);
    servers = servers.filter(s => s.accessVersion <= transactorAccessVersion);
    return new InitAccount({
      addr,
      data,
      players,
      servers,
      accessVersion: transactorAccessVersion,
      settleVersion: transactorSettleVersion,
    });
  }
  serialize(): Uint8Array {
    return serialize(InitAccount);
  }
  static deserialize(data: Uint8Array) {
    return deserialize(InitAccount, data);
  }
}

export interface IHandler {
  handleEvent(context: GameContext, event: GameEvent): Promise<void>;

  initState(context: GameContext, initAccount: InitAccount): Promise<void>;
}

export class Handler implements IHandler {

  #encryptor: IEncryptor;
  #instance: WebAssembly.Instance;

  constructor(instance: WebAssembly.Instance, encryptor: IEncryptor) {
    this.#encryptor = encryptor;
    this.#instance = instance;
  }

  static async initialize(gameBundle: GameBundle, encryptor: IEncryptor): Promise<Handler> {
    const importObject = {
      imports: {
        memory: new WebAssembly.Memory({
          shared: true,
          maximum: 100,
          initial: 100,
        })
      }
    };
    const initiatedSource = await WebAssembly.instantiateStreaming(fetch(gameBundle.uri), importObject);
    return new Handler(initiatedSource.instance, encryptor);
  }

  async handleEvent(context: GameContext, event: GameEvent) {
    this.generalPreHandleEvent(context, event);
    this.customHandleEvent(context, event);
    this.generalPostHandleEvent(context, event);
  }

  async initState(context: GameContext, initAccount: InitAccount) {
    this.generalPreInitState(context, initAccount);
    this.customInitState(context, initAccount);
    this.generalPostInitState(context, initAccount);
  }

  async generalPreInitState(_context: GameContext, _initAccount: InitAccount) { }

  async generalPostInitState(_context: GameContext, _initAccount: InitAccount) { }

  async generalPreHandleEvent(context: GameContext, event: GameEvent) { }

  async generalPostHandleEvent(context: GameContext, event: GameEvent) { }

  async customInitState(context: GameContext, initAccount: InitAccount) {
    const exports = this.#instance.exports;
    const effect = Effect.fromContext(context);
  }

  async customHandleEvent(context: GameContext, event: GameEvent) { }
}
