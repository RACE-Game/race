import { PublicKey } from '@solana/web3.js';
import * as borsh from 'borsh';
import { ExtendedReader, ExtendedWriter } from './utils'
import RaceCore, { VoteType } from 'race-sdk-core';
import { Buffer } from 'buffer';

export interface IPlayerState {
  isInitialized: boolean;
  nick: string;
  pfpKey?: PublicKey;
};

export interface IPlayerJoin {
  key: PublicKey;
  balance: bigint;
  position: number;
  accessVersion: bigint;
}

export interface IServerJoin {
  key: PublicKey;
  endpoint: string;
  accessVersion: bigint;
}

export interface IVote {
  voterKey: PublicKey;
  voteeKey: PublicKey;
  voteType: VoteType
}

export interface IGameReg {
  title: string;
  gameKey: PublicKey;
  bundleKey: PublicKey;
  regTime: bigint;
}

export interface IRegistryState {
  isInitialized: boolean;
  isPrivate: boolean;
  size: number,
  ownerKey: PublicKey,
  games: IGameReg[];
}

export interface IGameState {
  isInitialized: boolean;
  title: string;
  bundleKey: PublicKey;
  stakeKey: PublicKey;
  ownerKey: PublicKey;
  tokenKey: PublicKey;
  minDeposit: bigint;
  maxDeposit: bigint;
  transactorKey: PublicKey | undefined;
  accessVersion: bigint;
  settleVersion: bigint;
  maxPlayers: number;
  players: IPlayerJoin[];
  servers: IServerJoin[];
  dataLen: number;
  data: number[];
  votes: IVote[];
  unlockTime: bigint | undefined;
};

export interface IServerState {
  isInitialized: boolean;
  key: PublicKey;
  ownerKey: PublicKey;
  endpoint: string;
};

export class PlayerState implements IPlayerState {
  isInitialized!: boolean;
  nick!: string;
  pfpKey?: PublicKey;

  constructor(fields: IPlayerState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(borsh.serialize(playerStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Buffer): PlayerState {
    return borsh.deserializeUnchecked(playerStateSchema, PlayerState, data, ExtendedReader)
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
        ['pfpKey', { kind: 'option', type: 'publicKey' }],
      ],
    },
  ],
]);

export class Vote implements IVote {
  voterKey!: PublicKey;
  voteeKey!: PublicKey;
  voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields)
  }
  standardize(): RaceCore.Vote {
    return {
      voter: this.voterKey.toBase58(),
      votee: this.voteeKey.toBase58(),
      voteType: this.voteType
    }
  }
}

export class ServerJoin implements IServerJoin {
  key!: PublicKey;
  endpoint!: string;
  accessVersion!: bigint;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields)
  }
  standardize(): RaceCore.ServerJoin {
    return {
      addr: this.key.toBase58(),
      endpoint: this.endpoint,
      accessVersion: this.accessVersion,
    }
  }
}

export class PlayerJoin implements IPlayerJoin {
  key!: PublicKey;
  balance!: bigint;
  position!: number;
  accessVersion!: bigint;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields)
  }
  standardize(): RaceCore.PlayerJoin {
    return {
      addr: this.key.toBase58(),
      position: this.position,
      balance: this.balance,
      accessVersion: this.accessVersion,
    }
  }
}


export class GameState implements IGameState {
  isInitialized!: boolean;
  title!: string;
  bundleKey!: PublicKey;
  stakeKey!: PublicKey;
  ownerKey!: PublicKey;
  tokenKey!: PublicKey;
  minDeposit!: bigint;
  maxDeposit!: bigint;
  transactorKey: PublicKey | undefined;
  accessVersion!: bigint;
  settleVersion!: bigint;
  maxPlayers!: number;
  players!: PlayerJoin[];
  servers!: ServerJoin[];
  dataLen!: number;
  data!: number[];
  votes!: Vote[];
  unlockTime: bigint | undefined;

  constructor(fields: IGameState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(
      borsh.serialize(gameStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Buffer): GameState {
    return borsh.deserializeUnchecked(gameStateSchema, GameState, data, ExtendedReader)
  }

  generalize(addr: PublicKey): RaceCore.GameAccount {
    return {
      addr: addr.toBase58(),
      title: this.title,
      bundleAddr: this.bundleKey.toBase58(),
      ownerAddr: this.ownerKey.toBase58(),
      tokenAddr: this.tokenKey.toBase58(),
      deposits: [],
      minDeposit: this.minDeposit,
      maxDeposit: this.maxDeposit,
      transactorAddr: this.transactorKey?.toBase58(),
      accessVersion: this.accessVersion,
      settleVersion: this.settleVersion,
      maxPlayers: this.maxPlayers,
      players: this.players.map(p => p.standardize()),
      servers: this.servers.map(s => s.standardize()),
      dataLen: this.dataLen,
      data: this.data,
      votes: this.votes.map(v => v.standardize()),
      unlockTime: this.unlockTime,
    }
  }
}

const gameStateSchema = new Map<Function, any>([
  [
    PlayerJoin, {
      kind: 'struct',
      fields: [
        ['key', 'publicKey'],
        ['balance', 'bigint'],
        ['position', 'u32'],
        ['accessVersion', 'bigint']

      ]
    }
  ],
  [
    Vote, {
      kind: 'struct',
      fields: [
        ['voterKey', 'publicKey'],
        ['voteeKey', 'publicKey'],
        ['voteType', 'u8']
      ]
    }
  ],
  [
    ServerJoin, {
      kind: 'struct',
      fields: [
        ['key', 'publicKey'],
        ['endpoint', 'string'],
        ['accessVersion', 'bigint'],
      ]
    }
  ],
  [
    GameState, {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['title', 'string'],
        ['bundleKey', 'publicKey'],
        ['stakeKey', 'publicKey'],
        ['ownerKey', 'publicKey'],
        ['tokenKey', 'publicKey'],
        ['minDeposit', 'bigint'],
        ['maxDeposit', 'bigint'],
        ['transactorKey',
          { kind: 'option', type: 'publicKey' }],
        ['accessVersion', 'bigint'],
        ['settleVersion', 'bigint'],
        ['maxPlayers', 'u8'],
        ['players', [PlayerJoin]],
        ['servers', [ServerJoin]],
        ['dataLen', 'u32'],
        ['data', 'bytes'],
        ['votes', [Vote]],
        ['unlockTime',
          { kind: 'option', type: 'bigint' }]
      ]
    }
  ]
]);

export class GameReg implements IGameReg {
  title!: string;
  gameKey!: PublicKey;
  bundleKey!: PublicKey;
  regTime!: bigint;
  constructor(fields: IGameReg) {
    Object.assign(this, fields)
  }
  generalize(): RaceCore.GameRegistration {
    return {
      title: this.title,
      addr: this.gameKey.toBase58(),
      bundleAddr: this.bundleKey.toBase58(),
      regTime: this.regTime
    };
  }
}

export class RegistryState implements IRegistryState {
  isInitialized!: boolean;
  isPrivate!: boolean;
  size!: number;
  ownerKey!: PublicKey;
  games!: GameReg[];
  constructor(fields: IRegistryState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(
      borsh.serialize(registryStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Buffer): RegistryState {
    return borsh.deserializeUnchecked(registryStateSchema, RegistryState, data, ExtendedReader)
  }

  generalize(addr: PublicKey): RaceCore.RegistrationAccount {
    return {
      addr: addr.toBase58(),
      isPrivate: this.isPrivate,
      size: this.size,
      owner: this.ownerKey.toBase58(),
      games: this.games.map((g) => g.generalize())
    }
  }
}

const registryStateSchema = new Map<Function, any>([
  [
    GameReg, {
      kind: 'struct',
      fields: [
        ['title', 'string'],
        ['gameKey', 'publicKey'],
        ['bundleKey', 'publicKey'],
        ['regTime', 'bigint'],
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
        ['ownerKey', 'publicKey'],
        ['games', [GameReg]]
      ]
    }
  ]]);

export class ServerState implements IServerState {
  isInitialized!: boolean;
  key!: PublicKey;
  ownerKey!: PublicKey;
  endpoint!: string;

  constructor(fields: IServerState) {
    Object.assign(this, fields)
  }

  serialize(): Buffer {
    return Buffer.from(
      borsh.serialize(serverStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Buffer): ServerState {
    return borsh.deserializeUnchecked(serverStateSchema, ServerState, data, ExtendedReader)
  }

  generalize(): RaceCore.ServerAccount {
    return {
      addr: this.ownerKey.toBase58(),
      endpoint: this.endpoint
    }
  }
}

const serverStateSchema = new Map([
  [
    ServerState, {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['key', 'publicKey'],
        ['ownerKey', 'publicKey'],
        ['endpoint', 'string'],
      ]
    }
  ]]);
