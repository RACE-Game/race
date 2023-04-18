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

export interface IGameState {
  isInitalized: boolean;
  title: string;
  bundle: PublicKey;
  stake: PublicKey;
  owner: PublicKey;
  token: PublicKey;
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

export class GameState {
  players!: PlayerJoin[]

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
        ['voteType', { kind: 'enum', values: voteTypes }]
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
        ['players', [PlayerJoin]]
      ]
    }
  ]
]);
