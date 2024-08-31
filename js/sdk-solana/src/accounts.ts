import { PublicKey } from '@solana/web3.js';
import * as _ from 'borsh';
import { publicKeyExt } from './utils';
import * as RaceCore from '@race-foundation/sdk-core';
import { VoteType, EntryType } from '@race-foundation/sdk-core';
import { deserialize, serialize, field, option, array, struct, enums, variant } from '@race-foundation/borsh';

export interface IPlayerState {
  isInitialized: boolean;
  nick: string;
  pfpKey?: PublicKey;
}

export interface IPlayerJoin {
  key: PublicKey;
  balance: bigint;
  position: number;
  accessVersion: bigint;
  verifyKey: string;
}

export interface IServerJoin {
  key: PublicKey;
  endpoint: string;
  accessVersion: bigint;
  verifyKey: string;
}

export interface IVote {
  voterKey: PublicKey;
  voteeKey: PublicKey;
  voteType: VoteType;
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
  size: number;
  ownerKey: PublicKey;
  games: IGameReg[];
}

export interface IGameState {
  isInitialized: boolean;
  version: string;
  title: string;
  bundleKey: PublicKey;
  stakeKey: PublicKey;
  ownerKey: PublicKey;
  tokenKey: PublicKey;
  transactorKey: PublicKey | undefined;
  accessVersion: bigint;
  settleVersion: bigint;
  maxPlayers: number;
  players: IPlayerJoin[];
  servers: IServerJoin[];
  dataLen: number;
  data: Uint8Array;
  votes: IVote[];
  unlockTime: bigint | undefined;
  entryType: EntryType;
  recipientAddr: PublicKey;
  checkpoint: Uint8Array;
}

export interface IServerState {
  isInitialized: boolean;
  key: PublicKey;
  ownerKey: PublicKey;
  endpoint: string;
}

export interface IRecipientState {
  readonly addr: PublicKey;
  readonly capAddr: PublicKey | undefined;
  readonly slots: IRecipientSlot[];
}

const RECIPIENT_SLOT_TYPE = {
  Nft: 0,
  Token: 1
} as const;

type RecipientSlotType = (typeof RECIPIENT_SLOT_TYPE)[keyof typeof RECIPIENT_SLOT_TYPE];

export interface IRecipientSlot {
  readonly id: number;
  readonly slotType: RecipientSlotType;
  readonly tokenAddr: PublicKey;
  readonly stakeAddr: PublicKey;
  readonly shares: IRecipientSlotShare[];
}

export interface IRecipientSlotShare {
  readonly owner: RecipientSlotOwner;
  readonly weights: number;
  readonly claimAmount: bigint;
}

export class PlayerState implements IPlayerState {
  @field('bool')
  isInitialized!: boolean;
  @field('string')
  nick!: string;
  @field(option(publicKeyExt))
  pfpKey?: PublicKey;

  constructor(fields: IPlayerState) {
    Object.assign(this, fields);
  }

  serialize(): Uint8Array {
    return serialize(this);
  }

  static deserialize(data: Uint8Array): PlayerState {
    return deserialize(PlayerState, data);
  }

  generalize(addr: PublicKey): RaceCore.PlayerProfile {
    return new RaceCore.PlayerProfile({
      addr: addr.toBase58(),
      nick: this.nick,
      pfp: this.pfpKey?.toBase58(),
    });
  }
}

export class Vote implements IVote {
  @field(publicKeyExt)
  voterKey!: PublicKey;
  @field(publicKeyExt)
  voteeKey!: PublicKey;
  @field('u8')
  voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields);
  }
  generalize(): RaceCore.Vote {
    return new RaceCore.Vote({
      voter: this.voterKey.toBase58(),
      votee: this.voteeKey.toBase58(),
      voteType: this.voteType,
    });
  }
}

export class ServerJoin implements IServerJoin {
  @field(publicKeyExt)
  key!: PublicKey;
  @field('string')
  endpoint!: string;
  @field('u64')
  accessVersion!: bigint;
  @field('string')
  verifyKey!: string;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields);
  }
  generalize(): RaceCore.ServerJoin {
    return new RaceCore.ServerJoin({
      addr: this.key.toBase58(),
      endpoint: this.endpoint,
      accessVersion: this.accessVersion,
      verifyKey: this.verifyKey,
    });
  }
}

export class PlayerJoin implements IPlayerJoin {
  @field(publicKeyExt)
  key!: PublicKey;
  @field('u64')
  balance!: bigint;
  @field('u16')
  position!: number;
  @field('u64')
  accessVersion!: bigint;
  @field('string')
  verifyKey!: string;

  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields);
  }
  generalize(): RaceCore.PlayerJoin {
    return new RaceCore.PlayerJoin({
      addr: this.key.toBase58(),
      position: this.position,
      balance: this.balance,
      accessVersion: this.accessVersion,
      verifyKey: this.verifyKey,
    });
  }
}

export class GameState implements IGameState {
  @field('bool')
  isInitialized!: boolean;
  @field('string')
  version!: string;
  @field('string')
  title!: string;
  @field(publicKeyExt)
  bundleKey!: PublicKey;
  @field(publicKeyExt)
  stakeKey!: PublicKey;
  @field(publicKeyExt)
  ownerKey!: PublicKey;
  @field(publicKeyExt)
  tokenKey!: PublicKey;
  @field(option(publicKeyExt))
  transactorKey: PublicKey | undefined;
  @field('u64')
  accessVersion!: bigint;
  @field('u64')
  settleVersion!: bigint;
  @field('u16')
  maxPlayers!: number;
  @field(array(struct(PlayerJoin)))
  players!: PlayerJoin[];
  @field(array(struct(ServerJoin)))
  servers!: ServerJoin[];
  @field('u32')
  dataLen!: number;
  @field('u8-array')
  data!: Uint8Array;
  @field(array(struct(Vote)))
  votes!: Vote[];
  @field(option('u64'))
  unlockTime: bigint | undefined;
  @field(enums(EntryType))
  entryType!: EntryType;
  @field(publicKeyExt)
  recipientAddr!: PublicKey;
  @field('u8-array')
  checkpoint!: Uint8Array;

  constructor(fields: IGameState) {
    Object.assign(this, fields);
  }

  serialize(): Uint8Array {
    return serialize(this);
  }

  static deserialize(data: Uint8Array): GameState {
    return deserialize(GameState, data);
  }

  generalize(addr: PublicKey): RaceCore.GameAccount {

    return new RaceCore.GameAccount({
      addr: addr.toBase58(),
      title: this.title,
      bundleAddr: this.bundleKey.toBase58(),
      ownerAddr: this.ownerKey.toBase58(),
      tokenAddr: this.tokenKey.toBase58(),
      deposits: [],
      transactorAddr: this.transactorKey?.toBase58(),
      accessVersion: this.accessVersion,
      settleVersion: this.settleVersion,
      maxPlayers: this.maxPlayers,
      players: this.players.map(p => p.generalize()),
      servers: this.servers.map(s => s.generalize()),
      dataLen: this.dataLen,
      data: this.data,
      votes: this.votes.map(v => v.generalize()),
      unlockTime: this.unlockTime,
      entryType: this.entryType,
      recipientAddr: this.recipientAddr.toBase58(),
      checkpoint: RaceCore.Checkpoint.fromRaw(this.checkpoint),
    });
  }
}

export class GameReg implements IGameReg {
  @field('string')
  title!: string;
  @field(publicKeyExt)
  gameKey!: PublicKey;
  @field(publicKeyExt)
  bundleKey!: PublicKey;
  @field('u64')
  regTime!: bigint;
  constructor(fields: IGameReg) {
    Object.assign(this, fields);
  }
  generalize(): RaceCore.GameRegistration {
    return new RaceCore.GameRegistration({
      title: this.title,
      addr: this.gameKey.toBase58(),
      bundleAddr: this.bundleKey.toBase58(),
      regTime: this.regTime,
    });
  }
}

export class RegistryState implements IRegistryState {
  @field('bool')
  isInitialized!: boolean;
  @field('bool')
  isPrivate!: boolean;
  @field('u16')
  size!: number;
  @field(publicKeyExt)
  ownerKey!: PublicKey;
  @field(array(struct(GameReg)))
  games!: GameReg[];
  constructor(fields: IRegistryState) {
    Object.assign(this, fields);
  }

  serialize(): Uint8Array {
    return serialize(this);
  }

  static deserialize(data: Uint8Array): RegistryState {
    return deserialize(RegistryState, data);
  }

  generalize(addr: PublicKey): RaceCore.RegistrationAccount {
    return new RaceCore.RegistrationAccount({
      addr: addr.toBase58(),
      isPrivate: this.isPrivate,
      size: this.size,
      owner: this.ownerKey.toBase58(),
      games: this.games.map(g => g.generalize()),
    });
  }
}

export class ServerState implements IServerState {
  @field('bool')
  isInitialized!: boolean;
  @field(publicKeyExt)
  key!: PublicKey;
  @field(publicKeyExt)
  ownerKey!: PublicKey;
  @field('string')
  endpoint!: string;

  constructor(fields: IServerState) {
    Object.assign(this, fields);
  }

  serialize(): Uint8Array {
    return serialize(this);
  }

  static deserialize(data: Uint8Array): ServerState {
    return deserialize(this, data);
  }

  generalize(): RaceCore.ServerAccount {
    return new RaceCore.ServerAccount({
      addr: this.ownerKey.toBase58(),
      endpoint: this.endpoint,
    });
  }
}

export abstract class RecipientSlotOwner {}

@variant(0)
export class RecipientSlotOwnerUnassigned extends RecipientSlotOwner {
  @field('string')
  identifier!: string;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(1)
export class RecipientSlotOwnerAssigned extends RecipientSlotOwner {
  @field(publicKeyExt)
  addr!: PublicKey;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

export class RecipientSlotShare implements IRecipientSlotShare {
  @field(enums(RecipientSlotOwner))
  owner!: RecipientSlotOwner;
  @field('u16')
  weights!: number;
  @field('u64')
  claimAmount!: bigint;
  constructor(fields: IRecipientSlotShare) {
    Object.assign(this, fields);
  }

  generalize(): RaceCore.RecipientSlotShare {
    let owner: RecipientSlotOwner;
    if (this.owner instanceof RecipientSlotOwnerAssigned) {
      owner = new RaceCore.RecipientSlotOwnerAssigned({ addr: this.owner.addr.toBase58() });
    } else if (this.owner instanceof RecipientSlotOwnerUnassigned) {
      owner = new RaceCore.RecipientSlotOwnerUnassigned({ identifier: this.owner.identifier });
    } else {
      throw new Error('Invalid slot owner');
    }
    return new RaceCore.RecipientSlotShare({
      owner, weights: this.weights, claimAmount: this.claimAmount
    })
  }
}

export class RecipientSlot implements IRecipientSlot {
  @field('u8')
  id!: number;
  @field('u8')
  slotType!: RecipientSlotType;
  @field(publicKeyExt)
  tokenAddr!: PublicKey;
  @field(publicKeyExt)
  stakeAddr!: PublicKey;
  @field(array(struct(RecipientSlotShare)))
  shares!: RecipientSlotShare[];
  constructor(fields: IRecipientSlot) {
    Object.assign(this, fields);
  }

  generalize(balance: bigint): RaceCore.RecipientSlot {
    return new RaceCore.RecipientSlot({
      id: this.id,
      slotType: this.slotType,
      tokenAddr: this.tokenAddr.toBase58(),
      shares: this.shares.map(s => s.generalize()),
      balance,
    });
  }
}

export class RecipientState implements IRecipientState {
  @field(publicKeyExt)
  addr!: PublicKey;
  @field(option(publicKeyExt))
  capAddr: PublicKey | undefined;
  @field(array(struct(RecipientSlot)))
  slots!: RecipientSlot[];

  constructor(fields: IRecipientState) {
    Object.assign(this, fields);
  }

  serialize(): Uint8Array {
    return serialize(this);
  }

  static deserialize(data: Uint8Array): RecipientState {
    return deserialize(this, data);
  }

  generalize(slots: RaceCore.RecipientSlot[]): RaceCore.RecipientAccount {
    return new RaceCore.RecipientAccount({
      addr: this.addr.toBase58(),
      capAddr: this.capAddr?.toBase58(),
      slots
    });
  }
}
