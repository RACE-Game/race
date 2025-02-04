import { Address } from '@solana/web3.js'
import { publicKeyExt } from './utils'
import * as RaceCore from '@race-foundation/sdk-core'
import { deserialize, serialize, field, option, array, struct, enums, variant } from '@race-foundation/borsh'

export interface IPlayerState {
    isInitialized: boolean
    nick: string
    pfpKey?: Address
}

export interface IPlayerJoin {
    key: Address
    position: number
    accessVersion: bigint
    verifyKey: string
}

export interface IPlayerDeposit {
    key: Address
    amount: bigint
    accessVersion: bigint
    settleVersion: bigint
}

export interface IServerJoin {
    key: Address
    endpoint: string
    accessVersion: bigint
    verifyKey: string
}

export interface IVote {
    voterKey: Address
    voteeKey: Address
    voteType: VoteType
}

export interface IBonus {
    identifier: string
    tokenAddr: Address
    amount: bigint
}

export interface IGameReg {
    title: string
    gameKey: Address
    bundleKey: Address
    regTime: bigint
}

export interface IRegistryState {
    isInitialized: boolean
    isPrivate: boolean
    size: number
    ownerKey: Address
    games: IGameReg[]
}

export interface IGameState {
    isInitialized: boolean
    version: string
    title: string
    bundleKey: Address
    stakeKey: Address
    ownerKey: Address
    tokenKey: Address
    transactorKey: Address | undefined
    accessVersion: bigint
    settleVersion: bigint
    maxPlayers: number
    players: IPlayerJoin[]
    deposits: IPlayerDeposit[]
    servers: IServerJoin[]
    dataLen: number
    data: Uint8Array
    votes: IVote[]
    unlockTime: bigint | undefined
    entryType: AEntryType
    recipientAddr: Address
    checkpoint: Uint8Array
    entryLock: EntryLock
    bonuses: IBonus[]
}

export interface IServerState {
    isInitialized: boolean
    key: Address
    ownerKey: Address
    endpoint: string
}

export interface IRecipientState {
    isInitialized: boolean
    capAddr: Address | undefined
    slots: IRecipientSlot[]
}

type RecipientSlotType = RaceCore.Indices<typeof RaceCore.RECIPIENT_SLOT_TYPES>

export interface IRecipientSlot {
    readonly id: number
    readonly slotType: RecipientSlotType
    readonly tokenAddr: Address
    readonly stakeAddr: Address
    readonly shares: IRecipientSlotShare[]
}

export interface IRecipientSlotShare {
    readonly owner: RecipientSlotOwner
    readonly weights: number
    readonly claimAmount: bigint
}

export class PlayerState implements IPlayerState {
    @field('bool')
    isInitialized!: boolean
    @field('string')
    nick!: string
    @field(option(publicKeyExt))
    pfpKey?: Address

    constructor(fields: IPlayerState) {
        Object.assign(this, fields)
    }

    serialize(): Uint8Array {
        return serialize(this)
    }

    static deserialize(data: Uint8Array): PlayerState {
        return deserialize(PlayerState, data)
    }

    generalize(addr: Address): RaceCore.PlayerProfile {
        return {
            addr: addr,
            nick: this.nick,
            pfp: this.pfpKey,
        }
    }
}

type VoteType = RaceCore.Indices<typeof RaceCore.VOTE_TYPES>

export class Vote implements IVote {
    @field(publicKeyExt)
    voterKey!: Address
    @field(publicKeyExt)
    voteeKey!: Address
    @field('u8')
    voteType!: VoteType
    constructor(fields: IVote) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.Vote {
        return {
            voter: this.voterKey,
            votee: this.voteeKey,
            voteType: RaceCore.VOTE_TYPES[this.voteType],
        }
    }
}

export class ServerJoin implements IServerJoin {
    @field(publicKeyExt)
    key!: Address
    @field('string')
    endpoint!: string
    @field('u64')
    accessVersion!: bigint
    @field('string')
    verifyKey!: string
    constructor(fields: IServerJoin) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.ServerJoin {
        return {
            addr: this.key,
            endpoint: this.endpoint,
            accessVersion: this.accessVersion,
            verifyKey: this.verifyKey,
        }
    }
}

export class PlayerJoin implements IPlayerJoin {
    @field(publicKeyExt)
    key!: Address
    @field('u16')
    position!: number
    @field('u64')
    accessVersion!: bigint
    @field('string')
    verifyKey!: string

    constructor(fields: IPlayerJoin) {
        Object.assign(this, fields)
    }

    generalize(): RaceCore.PlayerJoin {
        return {
            addr: this.key,
            position: this.position,
            accessVersion: this.accessVersion,
            verifyKey: this.verifyKey,
        }
    }
}

export class PlayerDeposit implements IPlayerDeposit {
    @field(publicKeyExt)
    key!: Address
    @field('u64')
    amount!: bigint
    @field('u64')
    accessVersion!: bigint
    @field('u64')
    settleVersion!: bigint
    @field('u8')
    status!: RaceCore.DepositStatus

    constructor(fields: IPlayerJoin) {
        Object.assign(this, fields)
    }

    generalize(): RaceCore.PlayerDeposit {
        return {
            addr: this.key,
            amount: this.amount,
            accessVersion: this.accessVersion,
            settleVersion: this.settleVersion,
            status: this.status,
        }
    }
}

export class Bonus implements IBonus {
    @field('string')
    identifier!: string
    @field(publicKeyExt)
    stakeAddr!: Address
    @field(publicKeyExt)
    tokenAddr!: Address
    @field('u64')
    amount!: bigint

    constructor(fields: IBonus) {
        Object.assign(this, fields)
    }

    generalize(): RaceCore.Bonus {
        return {
            identifier: this.identifier,
            tokenAddr: this.tokenAddr,
            amount: this.amount,
        }
    }
}

type EntryLock = RaceCore.Indices<typeof RaceCore.ENTRY_LOCKS>

export abstract class AEntryType {
    static from(entryType: RaceCore.EntryType) {
        if (entryType.kind === 'cash') {
            return new EntryTypeCash(entryType)
        } else if (entryType.kind === 'ticket') {
            return new EntryTypeTicket(entryType)
        } else if (entryType.kind === 'gating') {
            return new EntryTypeGating(entryType)
        } else {
            return new EntryTypeDisabled(entryType)
        }
    }

    generalize(): RaceCore.EntryType {
        if (this instanceof EntryTypeCash) {
            return {
                kind: 'cash',
                minDeposit: this.minDeposit,
                maxDeposit: this.maxDeposit,
            }
        } else if (this instanceof EntryTypeTicket) {
            return {
                kind: 'ticket',
                amount: this.amount,
            }
        } else if (this instanceof EntryTypeGating) {
            return {
                kind: 'gating',
                collection: this.collection,
            }
        } else {
            return {
                kind: 'disabled',
            }
        }
    }
}

@variant(0)
export class EntryTypeCash extends AEntryType {
    @field('u64')
    minDeposit!: bigint
    @field('u64')
    maxDeposit!: bigint
    constructor(fields: RaceCore.Fields<EntryTypeCash>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeCash.prototype)
    }
}

@variant(1)
export class EntryTypeTicket extends AEntryType {
    @field('u64')
    amount!: bigint
    constructor(fields: RaceCore.Fields<EntryTypeTicket>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeTicket.prototype)
    }
}

@variant(2)
export class EntryTypeGating extends AEntryType {
    @field('string')
    collection!: string
    constructor(fields: RaceCore.Fields<EntryTypeGating>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeGating.prototype)
    }
}

@variant(3)
export class EntryTypeDisabled extends AEntryType {
    constructor(_: RaceCore.Fields<EntryTypeDisabled>) {
        super()
        Object.setPrototypeOf(this, EntryTypeDisabled.prototype)
    }
}

export class PlayerBalance {
    @field('u64')
    playerId!: bigint
    @field('u64')
    balance!: bigint
    constructor(fields: RaceCore.Fields<PlayerBalance>) {
        Object.assign(this, fields)
    }
}

export class GameState implements IGameState {
    @field('bool')
    isInitialized!: boolean
    @field('string')
    version!: string
    @field('string')
    title!: string
    @field(publicKeyExt)
    bundleKey!: Address
    @field(publicKeyExt)
    stakeKey!: Address
    @field(publicKeyExt)
    ownerKey!: Address
    @field(publicKeyExt)
    tokenKey!: Address
    @field(option(publicKeyExt))
    transactorKey: Address | undefined
    @field('u64')
    accessVersion!: bigint
    @field('u64')
    settleVersion!: bigint
    @field('u16')
    maxPlayers!: number
    @field(array(struct(PlayerJoin)))
    players!: PlayerJoin[]
    @field(array(struct(PlayerDeposit)))
    deposits!: PlayerDeposit[]
    @field(array(struct(ServerJoin)))
    servers!: ServerJoin[]
    @field('u32')
    dataLen!: number
    @field('u8-array')
    data!: Uint8Array
    @field(array(struct(Vote)))
    votes!: Vote[]
    @field(option('u64'))
    unlockTime: bigint | undefined
    @field(enums(AEntryType))
    entryType!: AEntryType
    @field(publicKeyExt)
    recipientAddr!: Address
    @field('u8-array')
    checkpoint!: Uint8Array
    @field('u8')
    entryLock!: EntryLock
    @field(array(struct(Bonus)))
    bonuses!: Bonus[]
    @field(array(struct(PlayerBalance)))
    balances!: PlayerBalance[]

    constructor(fields: IGameState) {
        Object.assign(this, fields)
    }

    serialize(): Uint8Array {
        return serialize(this)
    }

    static deserialize(data: Uint8Array): GameState {
        return deserialize(GameState, data)
    }

    generalize(addr: Address): RaceCore.GameAccount {
        let checkpointOnChain = undefined
        if (this.checkpoint.length !== 0) {
            checkpointOnChain = RaceCore.CheckpointOnChain.fromRaw(this.checkpoint)
        }

        return {
            addr: addr,
            title: this.title,
            bundleAddr: this.bundleKey,
            ownerAddr: this.ownerKey,
            tokenAddr: this.tokenKey,
            transactorAddr: this.transactorKey,
            accessVersion: this.accessVersion,
            settleVersion: this.settleVersion,
            maxPlayers: this.maxPlayers,
            players: this.players.map(p => p.generalize()),
            deposits: this.deposits.map(d => d.generalize()),
            servers: this.servers.map(s => s.generalize()),
            dataLen: this.dataLen,
            data: this.data,
            votes: this.votes.map(v => v.generalize()),
            unlockTime: this.unlockTime,
            entryType: this.entryType.generalize(),
            recipientAddr: this.recipientAddr,
            checkpointOnChain,
            entryLock: RaceCore.ENTRY_LOCKS[this.entryLock],
            bonuses: this.bonuses.map(b => b.generalize()),
            balances: this.balances,
        }
    }
}

export class GameReg implements IGameReg {
    @field('string')
    title!: string
    @field(publicKeyExt)
    gameKey!: Address
    @field(publicKeyExt)
    bundleKey!: Address
    @field('u64')
    regTime!: bigint
    constructor(fields: IGameReg) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.GameRegistration {
        return {
            title: this.title,
            addr: this.gameKey,
            bundleAddr: this.bundleKey,
            regTime: this.regTime,
        }
    }
}

export class RegistryState implements IRegistryState {
    @field('bool')
    isInitialized!: boolean
    @field('bool')
    isPrivate!: boolean
    @field('u16')
    size!: number
    @field(publicKeyExt)
    ownerKey!: Address
    @field(array(struct(GameReg)))
    games!: GameReg[]
    constructor(fields: IRegistryState) {
        Object.assign(this, fields)
    }

    serialize(): Uint8Array {
        return serialize(this)
    }

    static deserialize(data: Uint8Array): RegistryState {
        return deserialize(RegistryState, data)
    }

    generalize(addr: Address): RaceCore.RegistrationAccount {
        return {
            addr,
            isPrivate: this.isPrivate,
            size: this.size,
            owner: this.ownerKey,
            games: this.games.map(g => g.generalize()),
        }
    }
}

export class ServerState implements IServerState {
    @field('bool')
    isInitialized!: boolean
    @field(publicKeyExt)
    key!: Address
    @field(publicKeyExt)
    ownerKey!: Address
    @field('string')
    endpoint!: string

    constructor(fields: IServerState) {
        Object.assign(this, fields)
    }

    serialize(): Uint8Array {
        return serialize(this)
    }

    static deserialize(data: Uint8Array): ServerState {
        return deserialize(this, data)
    }

    generalize(): RaceCore.ServerAccount {
        return {
            addr: this.ownerKey,
            endpoint: this.endpoint,
        }
    }
}

export abstract class RecipientSlotOwner {}

@variant(0)
export class RecipientSlotOwnerUnassigned extends RecipientSlotOwner {
    @field('string')
    identifier!: string
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
}

@variant(1)
export class RecipientSlotOwnerAssigned extends RecipientSlotOwner {
    @field(publicKeyExt)
    addr!: Address
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
}

export class RecipientSlotShare implements IRecipientSlotShare {
    @field(enums(RecipientSlotOwner))
    owner!: RecipientSlotOwner
    @field('u16')
    weights!: number
    @field('u64')
    claimAmount!: bigint
    constructor(fields: IRecipientSlotShare) {
        Object.assign(this, fields)
    }

    generalize(): RaceCore.RecipientSlotShare {
        let owner: RaceCore.RecipientSlotOwner
        if (this.owner instanceof RecipientSlotOwnerAssigned) {
            owner = {
                kind: 'assigned',
                addr: this.owner.addr,
            }
        } else if (this.owner instanceof RecipientSlotOwnerUnassigned) {
            owner = {
                kind: 'unassigned',
                identifier: this.owner.identifier,
            }
        } else {
            throw new Error('Invalid slot owner')
        }
        return {
            owner,
            weights: this.weights,
            claimAmount: this.claimAmount,
        }
    }
}

export class RecipientSlot implements IRecipientSlot {
    @field('u8')
    id!: number
    @field('u8')
    slotType!: RecipientSlotType
    @field(publicKeyExt)
    tokenAddr!: Address
    @field(publicKeyExt)
    stakeAddr!: Address
    @field(array(struct(RecipientSlotShare)))
    shares!: RecipientSlotShare[]
    constructor(fields: IRecipientSlot) {
        Object.assign(this, fields)
    }

    generalize(balance: bigint): RaceCore.RecipientSlot {
        return {
            id: this.id,
            slotType: RaceCore.RECIPIENT_SLOT_TYPES[this.slotType],
            tokenAddr: this.tokenAddr,
            shares: this.shares.map(s => s.generalize()),
            balance,
        }
    }
}

export class RecipientState implements IRecipientState {
    @field('bool')
    isInitialized!: boolean
    @field(option(publicKeyExt))
    capAddr: Address | undefined
    @field(array(struct(RecipientSlot)))
    slots!: RecipientSlot[]

    constructor(fields: IRecipientState) {
        Object.assign(this, fields)
    }

    serialize(): Uint8Array {
        return serialize(this)
    }

    static deserialize(data: Uint8Array): RecipientState {
        return deserialize(this, data)
    }

    generalize(addr: string, slots: RaceCore.RecipientSlot[]): RaceCore.RecipientAccount {
        return {
            addr,
            capAddr: this.capAddr,
            slots,
        }
    }
}
