import { PublicKey } from '@solana/web3.js';
import * as borsh from 'borsh';
import { ExtendedReader, ExtendedWriter } from './utils'
import { voteTypes, VoteType } from 'race-sdk-core';

export interface IPlayerState {
  isInitialized: boolean;
  nick: string;
  pfp?: PublicKey;
};

export interface IPlayerJoin {
  addr: PublicKey;
  balance: bigint;
  position: number;
  accessVersion: bigint;
}

export interface IServerJoin {
  addr: PublicKey;
  endpoint: string;
  accessVersion: bigint;
}

export interface IVote {
  voter: PublicKey;
  votee: PublicKey;
  voteType: VoteType
}

export interface IGameReg {
  title: string;
  addr: PublicKey;
  bundleAddr: PublicKey;
  regTime: bigint;
}

export interface IRegistryState {
  isInitialized: boolean;
  isPrivate: boolean;
  size: number,
  owner: PublicKey,
  games: IGameReg[];
}

export interface IGameState {
  isInitalized: boolean;
  title: string;
  bundleAddr: PublicKey;
  stakeAddr: PublicKey;
  ownerAddr: PublicKey;
  tokenAddr: PublicKey;
  minDeposit: bigint;
  maxDeposit: bigint;
  transactorAddr: PublicKey | undefined;
  accessVersion: bigint;
  settleVersion: bigint;
  maxPlayers: number;
  players: IPlayerJoin[];
  servers: IServerJoin[];
  dataLen: number;
  data: Uint8Array;
  votes: IVote[];
  unlockTime: bigint | undefined;
};

export class PlayerState implements IPlayerState {
  isInitialized!: boolean;
  nick!: string;
  pfp?: PublicKey;

  constructor(fields: IPlayerState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(borsh.serialize(playerStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Uint8Array): PlayerState {
    return borsh.deserializeUnchecked(playerStateSchema, PlayerState, Buffer.from(data), ExtendedReader)
  }
}

const playerStateSchema = new Map([
  [
    PlayerState,
    {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['nick', 'string'],
        ['pfp', { kind: 'option', type: 'publicKey', },
        ],
      ],
    },
  ],
]);

export class Vote implements IVote {
  voter!: PublicKey;
  votee!: PublicKey;
  voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields)
  }
}

export class ServerJoin implements IServerJoin {
  addr!: PublicKey;
  endpoint!: string;
  accessVersion!: bigint;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields)
  }
}

export class PlayerJoin implements IPlayerJoin {
  addr!: PublicKey;
  balance!: bigint;
  position!: number;
  accessVersion!: bigint;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields)
  }
}


export class GameState {
  isInitialized!: boolean;
  title!: string;
  bundleAddr!: PublicKey;
  stakeAddr!: PublicKey;
  ownerAddr!: PublicKey;
  tokenAddr!: PublicKey;
  minDeposit!: bigint;
  maxDeposit!: bigint;
  transactorAddr: PublicKey | undefined;
  accessVersion!: bigint;
  settleVersion!: bigint;
  maxPlayers!: number;
  players!: IPlayerJoin[];
  servers!: IServerJoin[];
  dataLen!: number;
  data!: Uint8Array;
  votes!: IVote[];
  unlockTime: bigint | undefined;

  constructor(fields: IGameState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(
      borsh.serialize(gameStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Uint8Array): GameState {
    return borsh.deserializeUnchecked(gameStateSchema, GameState, Buffer.from(data), ExtendedReader)
  }
}

const gameStateSchema = new Map<Function, any>([
  [
    PlayerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'publicKey'],
        ['balance', 'u64'],
        ['accessVersion', 'u64'],
        ['position', 'u32']
      ]
    }
  ],
  [
    Vote, {
      kind: 'struct',
      fields: [
        ['voter', 'publicKey'],
        ['votee', 'publicKey'],
        // ['voteType', {
        //   kind: 'enum',
        //   values: [['ServerVoteTransactorDropOff', 0],
        //     ['ClientVoteTransactorDropOff', 1]]
        // }]
      ]
    }
  ],
  [
    PlayerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'publicKey'],
        ['balance', 'u64'],
        ['position', 'u32'],
        ['accessVersion', 'u64'],
      ]
    }
  ],
  [
    ServerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'publicKey'],
        ['endpoint', 'string'],
        ['accessVersion', 'u64'],
      ]
    }
  ],
  [
    GameState, {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['title', 'string'],
        ['bundleAddr', 'publicKey'],
        ['stakeAddr', 'publicKey'],
        ['ownerAddr', 'publicKey'],
        ['tokenAddr', 'publicKey'],
        ['minDeposit', 'u64'],
        ['maxDeposit', 'u64'],
        ['transactorAddr',
          { kind: 'option', type: 'publicKey' }],
        ['accessVersion', 'u64'],
        ['settleVersion', 'u64'],
        ['maxPlayers', 'u32'],
        ['players', [PlayerJoin]],
        ['servers', [ServerJoin]],
        ['dataLen', 'u32'],
        ['data', 'bytes'],
        ['votes', [Vote]],
        ['unlockTime',
          { kind: 'option', type: 'u64' }]
      ]
    }
  ]
]);

export class GameReg implements IGameReg {
  title!: string;
  addr!: PublicKey;
  bundleAddr!: PublicKey;
  regTime!: bigint;
  constructor(fields: IGameReg) {
    Object.assign(this, fields)
  }
}

export class RegistryState implements IRegistryState {
  isInitialized!: boolean;
  isPrivate!: boolean;
  size!: number;
  owner!: PublicKey;
  games!: IGameReg[];
  constructor(fields: IRegistryState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(
      borsh.serialize(registryStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Uint8Array): RegistryState {
    return borsh.deserializeUnchecked(registryStateSchema, RegistryState, Buffer.from(data), ExtendedReader)
  }
}

const registryStateSchema = new Map<Function, any>([
  [
    GameReg, {
      kind: 'struct',
      fields: [
        ['title', 'string'],
        ['addr', 'publicKey'],
        ['bundleAddr', 'publicKey'],
        ['regTime', 'u64'],
      ]
    }
  ],
  [
    RegistryState, {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['isPrivate', 'bool'],
        ['size', 'u16'],
        ['owner', 'publicKey'],
        ['games', [GameReg]]
      ]
    }
  ]]);
