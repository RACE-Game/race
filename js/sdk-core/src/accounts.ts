import { field, array, struct, option, enums, variant } from '@race-foundation/borsh';

export interface IPlayerJoin {
  readonly addr: string;
  readonly position: number;
  readonly balance: bigint;
  readonly accessVersion: bigint;
  readonly verifyKey: string;
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
  readonly verifyKey: string;
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
  readonly entryType: EntryType;
  readonly recipientAddr: string;
  readonly checkpoint: Uint8Array;
  readonly checkpointAccessVersion: bigint;
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

export interface IToken {
  readonly addr: string;
  readonly icon: string;
  readonly name: string;
  readonly symbol: string;
  readonly decimals: number;
}

export class Token implements IToken {
  readonly addr!: string;
  readonly icon!: string;
  readonly name!: string;
  readonly symbol!: string;
  readonly decimals!: number;
  constructor(fields: IToken) {
    Object.assign(this, fields);
  }
}

export class TokenWithBalance implements IToken {
  readonly addr!: string;
  readonly icon!: string;
  readonly name!: string;
  readonly symbol!: string;
  readonly decimals!: number;
  readonly amount!: bigint;
  readonly uiAmount!: string;
  constructor(token: IToken, amount: bigint) {
    Object.assign(this, token);
    this.amount = amount;
    this.uiAmount = (Number(amount) / Math.pow(10, token.decimals)).toLocaleString();
  }
}

export interface INft {
  readonly addr: string;
  readonly image: string;
  readonly name: string;
  readonly symbol: string;
  readonly collection: string | undefined;
  readonly metadata: any;
}

export interface IRecipientAccount {
  readonly addr: string;
  readonly capAddr: string | undefined;
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
  readonly tokenAddr: string;
  readonly shares: IRecipientSlotShare[];
}

export interface IRecipientSlotShare {
  readonly owner: RecipientSlotOwner;
  readonly weights: number;
  readonly claimAmount: bigint;
  readonly claimAmountCap: bigint;
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
  @field('string')
  addr!: string;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

export type EntryTypeKind =
  | 'Invalid'
  | 'Cash'
  | 'Ticket'
  | 'Gating';

export interface IEntryTypeKind {
  kind(): EntryTypeKind;
}

export abstract class EntryType {}

@variant(0)
export class EntryTypeCash extends EntryType implements IEntryTypeKind {
  @field('u64')
  minDeposit!: bigint;
  @field('u64')
  maxDeposit!: bigint;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
  kind(): EntryTypeKind {
    return 'Cash';
  }
}

@variant(1)
export class EntryTypeTicket extends EntryType implements IEntryTypeKind{
  @field('u8')
  slotId!: number;
  @field('u64')
  amount!: bigint;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
  kind(): EntryTypeKind {
    return 'Ticket';
  }
}

@variant(2)
export class EntryTypeGating extends EntryType implements IEntryTypeKind{
  @field('string')
  collection!: string;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
  kind(): EntryTypeKind {
    return 'Gating';
  }
}

export class Nft implements INft {
  @field('string')
  readonly addr!: string;
  @field('string')
  readonly image!: string;
  @field('string')
  readonly name!: string;
  @field('string')
  readonly symbol!: string;
  @field(option('string'))
  readonly collection: string | undefined;
  readonly metadata: any;
  constructor(fields: INft) {
    Object.assign(this, fields);
  }
}

export class ServerAccount implements IServerAccount {
  @field('string')
  readonly addr!: string;
  @field('string')
  readonly endpoint!: string;
  constructor(fields: IServerAccount) {
    Object.assign(this, fields);
  }
}

export class PlayerJoin implements IPlayerJoin {
  @field('string')
  readonly addr!: string;
  @field('u16')
  readonly position!: number;
  @field('u64')
  readonly balance!: bigint;
  @field('u64')
  readonly accessVersion!: bigint;
  @field('string')
  readonly verifyKey!: string;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields);
  }
}

export class ServerJoin implements IServerJoin {
  @field('string')
  readonly addr!: string;
  @field('string')
  readonly endpoint!: string;
  @field('u64')
  readonly accessVersion!: bigint;
  @field('string')
  readonly verifyKey!: string;
  constructor(fields: IServerJoin) {
    Object.assign(this, fields);
  }
}

export class PlayerDeposit implements IPlayerDeposit {
  @field('string')
  readonly addr!: string;
  @field('u64')
  readonly amount!: bigint;
  @field('u64')
  readonly settleVersion!: bigint;
  constructor(fields: IPlayerDeposit) {
    Object.assign(this, fields);
  }
}

export class Vote implements IVote {
  @field('string')
  readonly voter!: string;
  @field('string')
  readonly votee!: string;
  @field('u8')
  readonly voteType!: VoteType;
  constructor(fields: IVote) {
    Object.assign(this, fields);
  }
}

export class GameAccount implements IGameAccount {
  @field('string')
  readonly addr!: string;
  @field('string')
  readonly title!: string;
  @field('string')
  readonly bundleAddr!: string;
  @field('string')
  readonly tokenAddr!: string;
  @field('string')
  readonly ownerAddr!: string;
  @field('u64')
  readonly settleVersion!: bigint;
  @field('u64')
  readonly accessVersion!: bigint;
  @field(array(struct(PlayerJoin)))
  readonly players!: PlayerJoin[];
  @field(array(struct(PlayerDeposit)))
  readonly deposits!: PlayerDeposit[];
  @field(array(struct(ServerJoin)))
  readonly servers!: ServerJoin[];
  @field(option('string'))
  readonly transactorAddr: string | undefined;
  @field(array(struct(Vote)))
  readonly votes!: Vote[];
  @field(option('u64'))
  readonly unlockTime: bigint | undefined;
  @field('u16')
  readonly maxPlayers!: number;
  @field('u32')
  readonly dataLen!: number;
  @field('u8-array')
  readonly data!: Uint8Array;
  @field(enums(EntryType))
  readonly entryType!: EntryType;
  @field('string')
  readonly recipientAddr!: string;
  @field('u8-array')
  readonly checkpoint!: Uint8Array;
  @field('u64')
  readonly checkpointAccessVersion!: bigint;
  constructor(fields: IGameAccount) {
    Object.assign(this, fields);
  }
}

export class GameBundle implements IGameBundle {
  @field('string')
  readonly uri!: string;
  @field('string')
  readonly name!: string;
  @field('u8-array')
  readonly data!: Uint8Array;

  constructor(fields: IGameBundle) {
    Object.assign(this, fields);
  }
}

export class GameRegistration implements IGameRegistration {
  @field('string')
  readonly title!: string;
  @field('string')
  readonly addr!: string;
  @field('u64')
  readonly regTime!: bigint;
  @field('string')
  readonly bundleAddr!: string;
  constructor(fields: IGameRegistration) {
    Object.assign(this, fields);
  }
}

export class RegistrationAccount implements IRegistrationAccount {
  @field('string')
  readonly addr!: string;
  @field('bool')
  readonly isPrivate!: boolean;
  @field('u16')
  readonly size!: number;
  @field(option('string'))
  readonly owner!: string | undefined;
  @field(array(struct(GameRegistration)))
  readonly games!: GameRegistration[];
  constructor(fields: IRegistrationAccount) {
    Object.assign(this, fields);
  }
}

/**
 * The registration account data with games consolidated.
 */
export class RegistrationWithGames {
  readonly addr!: string;
  readonly isPrivate!: boolean;
  readonly size!: number;
  readonly owner: string | undefined;
  readonly games!: GameAccount[];
  constructor(fields: Object) {
    Object.assign(this, fields);
  }
}

export class PlayerProfile implements IPlayerProfile {
  @field('string')
  readonly addr!: string;
  @field('string')
  readonly nick!: string;
  @field(option('string'))
  readonly pfp: string | undefined;
  constructor(fields: IPlayerProfile) {
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
  @field('u64')
  claimAmountCap!: bigint;
  constructor(fields: IRecipientSlotShare) {
    Object.assign(this, fields);
  }
}

export class RecipientSlot implements IRecipientSlot {
  @field('u8')
  id!: number;
  @field('u8')
  slotType!: RecipientSlotType;
  @field('string')
  tokenAddr!: string;
  @field(array(struct(RecipientSlotShare)))
  shares!: IRecipientSlotShare[];
  constructor(fields: IRecipientSlot) {
    Object.assign(this, fields);
  }
}

export class RecipientAccount implements IRecipientAccount {
  @field('string')
  addr!: string;
  @field(option('string'))
  capAddr: string | undefined;
  @field(array(struct(RecipientSlot)))
  slots!: IRecipientSlot[];
  constructor(fields: IRecipientAccount) {
    Object.assign(this, fields);
  }
}
