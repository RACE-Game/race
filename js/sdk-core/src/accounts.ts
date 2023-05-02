import * as borsh from 'borsh';
import { Buffer } from 'buffer';
import { ExtendedReader, ExtendedWriter } from './utils';

export interface IPlayerJoin {
  readonly addr: string;
  readonly position: number;
  readonly balance: bigint;
  readonly accessVersion: bigint;
}

export interface IPlayerDeposit {
  readonly addr: string;
  readonly amount: bigint;
  readonly settleVersion: bigint;
}

export interface IServerJoin {
  readonly addr: string;
  readonly endpoint: string;
  readonly accessVersion: bigint;
}

export enum VoteType {
  ServerVoteTransactorDropOff = 0,
  ClientVoteTransactorDropOff = 1,
}

export interface IVote {
  readonly voter: string;
  readonly votee: string;
  readonly voteType: VoteType;
}

export interface IGameRegistration {
  readonly title: string;
  readonly addr: string;
  readonly regTime: bigint;
  readonly bundleAddr: string;
}

export interface IGameAccount {
  readonly addr: string;
  readonly title: string;
  readonly bundleAddr: string;
  readonly tokenAddr: string;
  readonly ownerAddr: string;
  readonly settleVersion: bigint;
  readonly accessVersion: bigint;
  readonly players: PlayerJoin[];
  readonly deposits: PlayerDeposit[];
  readonly servers: ServerJoin[];
  readonly transactorAddr: string | undefined;
  readonly votes: Vote[];
  readonly unlockTime: bigint | undefined;
  readonly maxPlayers: number;
  readonly dataLen: number;
  readonly data: Uint8Array;
  readonly minDeposit: bigint;
  readonly maxDeposit: bigint;
}

export interface IServerAccount {
  readonly addr: string;
  readonly endpoint: string;
}

export interface IGameBundle {
  readonly uri: string;
  readonly name: string;
  readonly data: Uint8Array;
}

export interface IPlayerProfile {
  readonly addr: string;
  readonly nick: string;
  readonly pfp: string | undefined;
}

export interface IRegistrationAccount {
  readonly addr: string;
  readonly isPrivate: boolean;
  readonly size: number;
  readonly owner: string | undefined;
  readonly games: GameRegistration[];
}

export class ServerAccount implements IServerAccount {
  readonly addr!: string;
  readonly endpoint!: string;
  constructor(fields: IServerAccount) {
    Object.assign(this, fields);
  }
  serialize(): Uint8Array {
    return borsh.serialize(ServerAccount.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(ServerAccount.schema, ServerAccount, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [
        ServerAccount,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['endpoint', 'string'],
          ],
        },
      ],
    ]);
  }
}

export class PlayerJoin implements IPlayerJoin {
  readonly addr!: string;
  readonly position!: number;
  readonly balance!: bigint;
  readonly accessVersion!: bigint;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [PlayerJoin,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['position', 'u16'],
            ['balance', 'bigint'],
            ['accessVersion', 'bigint'],
          ],
        }]
    ]);
  }
}

export class ServerJoin implements IServerJoin {
  readonly addr!: string;
  readonly endpoint!: string;
  readonly accessVersion!: bigint;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [
        ServerJoin,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['endpoint', 'string'],
            ['accessVersion', 'bigint'],
          ],
        },
      ]
    ])
  }
}

export class PlayerDeposit implements IPlayerDeposit {
  readonly addr!: string;
  readonly amount!: bigint;
  readonly settleVersion!: bigint;
  constructor(fields: IPlayerDeposit) {
    Object.assign(this, fields);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [
        PlayerDeposit,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['amount', 'bigint'],
            ['settleVersion', 'bigint'],
          ],
        },
      ]
    ]);
  }
}

export class Vote implements IVote {
  readonly voter!: string;
  readonly votee!: string;
  readonly voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [
        Vote,
        {
          kind: 'struct',
          fields: [
            ['voter', 'string'],
            ['votee', 'string'],
            ['voteType', 'u8'],
          ],
        },
      ]
    ])
  }
}

export class GameAccount implements IGameAccount {
  readonly addr!: string;
  readonly title!: string;
  readonly bundleAddr!: string;
  readonly tokenAddr!: string;
  readonly ownerAddr!: string;
  readonly settleVersion!: bigint;
  readonly accessVersion!: bigint;
  readonly players!: PlayerJoin[];
  readonly deposits!: PlayerDeposit[];
  readonly servers!: ServerJoin[];
  readonly transactorAddr: string | undefined;
  readonly votes!: Vote[];
  readonly unlockTime: bigint | undefined;
  readonly maxPlayers!: number;
  readonly dataLen!: number;
  readonly data!: Uint8Array;
  readonly minDeposit!: bigint;
  readonly maxDeposit!: bigint;
  constructor(fields: IGameAccount) {
    Object.assign(this, fields);
  }
  serialize(): Uint8Array {
    return borsh.serialize(GameAccount.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(GameAccount.schema, GameAccount, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map<Function, any>([
      ...PlayerJoin.schema,
      ...ServerJoin.schema,
      ...PlayerDeposit.schema,
      ...Vote.schema,
      [
        GameAccount,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['title', 'string'],
            ['bundleAddr', 'string'],
            ['tokenAddr', 'string'],
            ['ownerAddr', 'string'],
            ['settleVersion', 'bigint'],
            ['accessVersion', 'bigint'],
            ['players', [PlayerJoin]],
            ['deposits', [PlayerDeposit]],
            ['servers', [ServerJoin]],
            ['transactorAddr', { kind: 'option', type: 'string' }],
            ['votes', [Vote]],
            ['unlockTime', { kind: 'option', type: 'bigint' }],
            ['maxPlayers', 'u16'],
            ['minDeposit', 'bigint'],
            ['maxDeposit', 'bigint'],
            ['dataLen', 'u32'],
            ['data', 'bytes'],
          ],
        },
      ],
    ]);
  }
}

export class GameBundle implements IGameBundle {
  readonly uri!: string;
  readonly name!: string;
  readonly data!: Uint8Array;

  constructor(fields: IGameBundle) {
    Object.assign(this, fields);
  }
  serialize(): Uint8Array {
    return borsh.serialize(GameBundle.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(GameBundle.schema, GameBundle, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map<Function, any>([
      [
        GameBundle,
        {
          kind: 'struct',
          fields: [
            ['uri', 'string'],
            ['name', 'string'],
            ['data', 'bytes'],
          ],
        },
      ],
    ]);
  }
}

export class GameRegistration implements IGameRegistration {
  readonly title!: string;
  readonly addr!: string;
  readonly regTime!: bigint;
  readonly bundleAddr!: string;
  constructor(fields: IGameRegistration) {
    Object.assign(this, fields);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [GameRegistration,
        {
          kind: 'struct',
          fields: [
            ['title', 'string'],
            ['addr', 'string'],
            ['regTime', 'bigint'],
            ['bundleAddr', 'string'],
          ],
        }
      ]
    ]);
  }
}

export class RegistrationAccount implements IRegistrationAccount {
  readonly addr!: string;
  readonly isPrivate!: boolean;
  readonly size!: number;
  readonly owner!: string | undefined;
  readonly games!: GameRegistration[];
  constructor(fields: IRegistrationAccount) {
    Object.assign(this, fields);
  }
  serialize(): Uint8Array {
    return borsh.serialize(RegistrationAccount.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(RegistrationAccount.schema, RegistrationAccount, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map<Function, any>([
      ...GameRegistration.schema,
      [
        RegistrationAccount,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['isPrivate', 'bool'],
            ['size', 'u16'],
            ['owner', { kind: 'option', type: 'string' }],
            ['games', [GameRegistration]],
          ],
        },
      ],
    ]);
  }
}

export class PlayerProfile implements IPlayerProfile {
  readonly addr!: string;
  readonly nick!: string;
  readonly pfp: string | undefined;
  constructor(fields: IPlayerProfile) {
    Object.assign(this, fields);
  }
  serialize(): Uint8Array {
    return borsh.serialize(PlayerProfile.schema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(PlayerProfile.schema, PlayerProfile, Buffer.from(data), ExtendedReader);
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [
        PlayerProfile,
        {
          kind: 'struct',
          fields: [
            ['addr', 'string'],
            ['nick', 'string'],
            ['pfp', { kind: 'option', type: 'string' }],
          ],
        },
      ],
    ]);
  }
}
