import * as borsh from 'borsh';
import { GameAccount, IGameAccount, PlayerJoin, ServerJoin } from "./accounts";
import { GameEvent } from "./events";
import { GameContext } from "./game-context";
import { ExtendedReader, ExtendedWriter } from './utils';

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
  static createFromGameAccount(gameAccount: GameAccount, transactorAccessVersion: bigint, transactorSettleVersion: bigint): InitAccount {
    let { addr, players, servers, data } = gameAccount;
    players = players.filter(p => p.accessVersion <= transactorAccessVersion);
    servers = servers.filter(s => s.accessVersion <= transactorAccessVersion);
    return new InitAccount({
      addr, data, players, servers,
      accessVersion: transactorAccessVersion,
      settleVersion: transactorSettleVersion
    });
  }
  serialize(): Uint8Array {
    return borsh.serialize(InitAccount.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(InitAccount.schema, InitAccount, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      ...PlayerJoin.schema,
      ...ServerJoin.schema,
      [InitAccount, {
        fields: [
          ['addr', 'string'],
          ['players', [PlayerJoin]],
          ['servers', [ServerJoin]],
          ['data', 'bytes'],
          ['accessVersion', 'bigint'],
          ['settleVersion', 'bigint'],
        ]
      }]
    ])
  }
}

export interface IHandler {

  handleEvent(context: GameContext, event: GameEvent): Promise<void>;

  initState(context: GameContext, initAccount: InitAccount): Promise<void>;
}

export class Handler {
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

  async generalPreInitState(context: GameContext, initAccount: InitAccount) {

  }

  async generalPostInitState(_context: GameContext, _initAccount: InitAccount) { }

  async customInitState(context: GameContext, initAccount: InitAccount) {

  }

  async generalPreHandleEvent(context: GameContext, event: GameEvent) {
    if (event === 'Ready') {

    } else if (event[ShareSecrets] !== undefined) {

    }
  }

  async generalPostHandleEvent(context: GameContext, event: GameEvent) {

  }

  async customHandleEvent(context: GameContext, event: GameEvent) {

  }
}
