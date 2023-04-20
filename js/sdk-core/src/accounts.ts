import * as borsh from 'borsh';
import { Buffer } from 'buffer';
import { ExtendedReader, ExtendedWriter } from './utils';

export interface IPlayerJoin {
  readonly addr: string;
  readonly position: number;
  readonly balance: bigint;
  readonly accessVersion: bigint;
};

export interface IPlayerDeposit {
  readonly addr: string;
  readonly amount: bigint;
  readonly settleVersion: bigint;
};

export interface IServerJoin {
  readonly addr: string;
  readonly endpoint: string;
  readonly accessVersion: bigint;
};

export enum VoteType {
  ServerVoteTransactorDropOff = 0,
  ClientVoteTransactorDropOff = 1,
}

export interface IVote {
  readonly voter: string;
  readonly votee: string;
  readonly voteType: VoteType;
};

export interface IGameRegistration {
  readonly title: string;
  readonly addr: string;
  readonly regTime: bigint;
  readonly bundleAddr: string;
};

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
};

export interface IServerAccount {
  readonly addr: string;
  readonly endpoint: string;
};

export interface IGameBundle {
  readonly uri: string;
  readonly name: string;
  readonly data: Uint8Array;
};

export interface IPlayerProfile {
  readonly addr: string;
  readonly nick: string;
  readonly pfp: string | undefined;
};

export interface IRegistrationAccount {
  readonly addr: string;
  readonly isPrivate: boolean;
  readonly size: number;
  readonly owner: string | undefined;
  readonly games: GameRegistration[];
};

export class ServerAccount implements IServerAccount {
  readonly addr!: string;
  readonly endpoint!: string;
  constructor(fields: IServerAccount) {
    Object.assign(this, fields)
  }
  serialize(): Uint8Array {
    return borsh.serialize(serverAccountSchema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(serverAccountSchema, ServerAccount, Buffer.from(data), ExtendedReader)
  }
}

const serverAccountSchema = new Map([
  [
    ServerAccount, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['endpoint', 'string'],
      ]
    }
  ]
]);

export class PlayerJoin implements IPlayerJoin {
  readonly addr!: string;
  readonly position!: number;
  readonly balance!: bigint;
  readonly accessVersion!: bigint;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields)
  }
};

export class ServerJoin implements IServerJoin {
  readonly addr!: string;
  readonly endpoint!: string;
  readonly accessVersion!: bigint;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields)
  }
}

export class PlayerDeposit implements IPlayerDeposit {
  readonly addr!: string;
  readonly amount!: bigint;
  readonly settleVersion!: bigint;
  constructor(fields: IPlayerDeposit) {
    Object.assign(this, fields)
  }
}

export class Vote implements IVote {
  readonly voter!: string;
  readonly votee!: string;
  readonly voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields)
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
    Object.assign(this, fields)
  }
  serialize(): Uint8Array {
    return borsh.serialize(gameAccountSchema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(gameAccountSchema, GameAccount, Buffer.from(data), ExtendedReader)
  }
}

const gameAccountSchema = new Map<Function, any>([
  [
    PlayerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['position', 'u16'],
        ['balance', 'bigint'],
        ['accessVersion', 'bigint']
      ]
    }
  ],
  [
    ServerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['endpoint', 'string'],
        ['accessVersion', 'bigint'],
      ]
    }
  ],
  [
    PlayerDeposit, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['amount', 'bigint'],
        ['settleVersion', 'bigint'],
      ]
    }
  ],
  [
    Vote, {
      kind: 'struct',
      fields: [
        ['voter', 'string'],
        ['votee', 'string'],
        ['voteType', 'u8'],
      ]
    }
  ],
  [
    GameAccount, {
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
      ]
    }
  ]
]);

export class GameBundle implements IGameBundle {
  readonly uri!: string;
  readonly name!: string;
  readonly data!: Uint8Array;

  constructor(fields: IGameBundle) {
    Object.assign(this, fields)
  }
  serialize(): Uint8Array {
    return borsh.serialize(gameBundleSchema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(gameBundleSchema, GameBundle, Buffer.from(data), ExtendedReader)
  }
}

const gameBundleSchema = new Map<Function, any>([
  [
    GameBundle, {
      kind: 'struct',
      fields: [
        ['uri', 'string'],
        ['name', 'string'],
        ['data', 'bytes'],
      ]
    }
  ]
]);

export class GameRegistration implements IGameRegistration {
  readonly title!: string;
  readonly addr!: string;
  readonly regTime!: bigint;
  readonly bundleAddr!: string;
  constructor(fields: IGameRegistration) {
    Object.assign(this, fields)
  }
}

export class RegistrationAccount implements IRegistrationAccount {
  readonly addr!: string;
  readonly isPrivate!: boolean;
  readonly size!: number;
  readonly owner!: string | undefined;
  readonly games!: GameRegistration[];
  constructor(fields: IRegistrationAccount) {
    Object.assign(this, fields)
  }
  serialize(): Uint8Array {
    return borsh.serialize(registrationAccountSchema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(registrationAccountSchema, RegistrationAccount, Buffer.from(data), ExtendedReader)
  }
}

const registrationAccountSchema = new Map<Function, any>([
  [
    GameRegistration, {
      kind: 'struct',
      fields: [
        ['title', 'string'],
        ['addr', 'string'],
        ['regTime', 'bigint'],
        ['bundleAddr', 'string'],
      ]
    },
  ],
  [
    RegistrationAccount, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['isPrivate', 'bool'],
        ['size', 'u16'],
        ['owner', { kind: 'option', type: 'string' }],
        ['games', [GameRegistration]]
      ]
    }
  ]
]);

export class PlayerProfile implements IPlayerProfile {
  readonly addr!: string;
  readonly nick!: string;
  readonly pfp: string | undefined;
  constructor(fields: IPlayerProfile) {
    Object.assign(this, fields)
  }
  serialize(): Uint8Array {
    return borsh.serialize(playerProfileSchema, this, ExtendedWriter);
  }
  static deserialize(data: Uint8Array) {
    return borsh.deserialize(playerProfileSchema, PlayerProfile, Buffer.from(data), ExtendedReader)
  }
}

const playerProfileSchema = new Map([
  [
    PlayerProfile, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['nick', 'string'],
        ['pfp', { kind: 'option', type: 'string' }],
      ],
    }
  ]
]);
