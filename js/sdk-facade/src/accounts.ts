import { field, array, struct, option, enums, variant } from '@race-foundation/borsh'
import {
    CheckpointOnChain,
    ENTRY_LOCKS,
    Fields,
    Indices,
    RECIPIENT_SLOT_TYPES,
    VOTE_TYPES,
} from '@race-foundation/sdk-core'
import * as RaceCore from '@race-foundation/sdk-core'

type RecipientSlotType = Indices<typeof RECIPIENT_SLOT_TYPES>

export abstract class RecipientSlotOwner {
    generalize(): RaceCore.RecipientSlotOwner {
        if (this instanceof RecipientSlotOwnerUnassigned) {
            return { kind: 'unassigned', identifier: this.identifier }
        } else if (this instanceof RecipientSlotOwnerAssigned) {
            return { kind: 'assigned', addr: this.addr }
        } else {
            throw new Error('Invalid RecipientSlotOwner')
        }
    }
}

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
    @field('string')
    addr!: string
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
}

export type EntryTypeKind = 'Invalid' | 'Cash' | 'Ticket' | 'Gating' | 'Disabled'

export interface IEntryTypeKind {
    kind(): EntryTypeKind
}

export abstract class EntryType implements IEntryTypeKind {
    kind(): EntryTypeKind {
        return 'Invalid'
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
export class EntryTypeCash extends EntryType implements IEntryTypeKind {
    @field('u64')
    minDeposit!: bigint
    @field('u64')
    maxDeposit!: bigint
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeCash.prototype)
    }
    kind(): EntryTypeKind {
        return 'Cash'
    }
}

@variant(1)
export class EntryTypeTicket extends EntryType implements IEntryTypeKind {
    @field('u64')
    amount!: bigint
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeTicket.prototype)
    }
    kind(): EntryTypeKind {
        return 'Ticket'
    }
}

@variant(2)
export class EntryTypeGating extends EntryType implements IEntryTypeKind {
    @field('string')
    collection!: string
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, EntryTypeGating.prototype)
    }
    kind(): EntryTypeKind {
        return 'Gating'
    }
}

@variant(3)
export class EntryTypeDisabled extends EntryType implements IEntryTypeKind {
    constructor(_: any) {
        super()
        Object.setPrototypeOf(this, EntryTypeDisabled.prototype)
    }
    kind(): EntryTypeKind {
        return 'Disabled'
    }
}

export class Nft {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly image!: string
    @field('string')
    readonly name!: string
    @field('string')
    readonly symbol!: string
    @field(option('string'))
    readonly collection: string | undefined
    readonly metadata: any
    constructor(fields: Fields<Nft>) {
        Object.assign(this, fields)
    }
}

export class ServerAccount {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly endpoint!: string
    constructor(fields: Fields<ServerAccount>) {
        Object.assign(this, fields)
    }
}

export class PlayerJoin {
    @field('string')
    readonly addr!: string
    @field('u16')
    readonly position!: number
    @field('u64')
    readonly accessVersion!: bigint
    @field('string')
    readonly verifyKey!: string
    constructor(fields: Fields<PlayerJoin>) {
        Object.assign(this, fields)
    }
}

export class ServerJoin {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly endpoint!: string
    @field('u64')
    readonly accessVersion!: bigint
    @field('string')
    readonly verifyKey!: string
    constructor(fields: Fields<ServerJoin>) {
        Object.assign(this, fields)
    }
}

export class PlayerDeposit {
    @field('string')
    readonly addr!: string
    @field('u64')
    readonly amount!: bigint
    @field('u64')
    readonly accessVersion!: bigint
    @field('u64')
    readonly settleVersion!: bigint
    @field('u8')
    readonly status!: RaceCore.DepositStatus
    constructor(fields: Fields<PlayerDeposit>) {
        Object.assign(this, fields)
    }
}

export class Bonus {
    @field('string')
    identifier!: string
    @field('string')
    tokenAddr!: string
    @field('u64')
    amount!: bigint

    constructor(fields: Fields<Bonus>) {
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


export type VoteType = Indices<typeof VOTE_TYPES>

export class Vote {
    @field('string')
    readonly voter!: string
    @field('string')
    readonly votee!: string
    @field('u8')
    readonly voteType!: VoteType
    constructor(fields: Fields<Vote>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.Vote {
        return {
            voter: this.voter,
            votee: this.votee,
            voteType: VOTE_TYPES[this.voteType],
        }
    }
}

export type EntryLock = Indices<typeof ENTRY_LOCKS>


export class PlayerBalance {
    @field('u64')
    readonly playerId!: bigint
    @field('u64')
    readonly balance!: bigint
    constructor(fields: Fields<PlayerBalance>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.PlayerBalance {
        return {
            playerId: this.playerId,
            balance: this.balance,
        }
    }
}

export class GameAccount {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly title!: string
    @field('string')
    readonly bundleAddr!: string
    @field('string')
    readonly tokenAddr!: string
    @field('string')
    readonly ownerAddr!: string
    @field('u64')
    readonly settleVersion!: bigint
    @field('u64')
    readonly accessVersion!: bigint
    @field(array(struct(PlayerJoin)))
    readonly players!: PlayerJoin[]
    @field(array(struct(PlayerDeposit)))
    readonly deposits!: PlayerDeposit[]
    @field(array(struct(ServerJoin)))
    readonly servers!: ServerJoin[]
    @field(option('string'))
    readonly transactorAddr: string | undefined
    @field(array(struct(Vote)))
    readonly votes!: Vote[]
    @field(option('u64'))
    readonly unlockTime: bigint | undefined
    @field('u16')
    readonly maxPlayers!: number
    @field('u32')
    readonly dataLen!: number
    @field('u8-array')
    readonly data!: Uint8Array
    @field(enums(EntryType))
    readonly entryType!: EntryType
    @field('string')
    readonly recipientAddr!: string
    @field(option(struct(CheckpointOnChain)))
    readonly checkpointOnChain: CheckpointOnChain | undefined
    @field('u8')
    readonly entryLock!: EntryLock
    @field(array(struct(Bonus)))
    readonly bonuses!: Bonus[]
    @field(array(struct(PlayerBalance)))
    readonly balances!: PlayerBalance[]
    constructor(fields: Fields<GameAccount>) {
        Object.assign(this, fields)
    }
    generalize() {
        return {
            addr: this.addr,
            title: this.title,
            bundleAddr: this.bundleAddr,
            ownerAddr: this.ownerAddr,
            tokenAddr: this.tokenAddr,
            transactorAddr: this.transactorAddr,
            accessVersion: this.accessVersion,
            settleVersion: this.settleVersion,
            maxPlayers: this.maxPlayers,
            players: this.players,
            deposits: this.deposits,
            servers: this.servers,
            dataLen: this.dataLen,
            data: this.data,
            votes: this.votes.map(v => v.generalize()),
            unlockTime: this.unlockTime,
            entryType: this.entryType.generalize(),
            recipientAddr: this.recipientAddr,
            checkpointOnChain: this.checkpointOnChain,
            entryLock: RaceCore.ENTRY_LOCKS[this.entryLock],
            bonuses: this.bonuses,
            balances: this.balances,
        }
    }
}

export class GameBundle {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly uri!: string
    @field('string')
    readonly name!: string
    @field('u8-array')
    readonly data!: Uint8Array

    constructor(fields: Fields<GameBundle>) {
        Object.assign(this, fields)
    }
}

export class GameRegistration {
    @field('string')
    readonly title!: string
    @field('string')
    readonly addr!: string
    @field('u64')
    readonly regTime!: bigint
    @field('string')
    readonly bundleAddr!: string
    constructor(fields: Fields<GameRegistration>) {
        Object.assign(this, fields)
    }
}

export class RegistrationAccount {
    @field('string')
    readonly addr!: string
    @field('bool')
    readonly isPrivate!: boolean
    @field('u16')
    readonly size!: number
    @field(option('string'))
    readonly owner!: string | undefined
    @field(array(struct(GameRegistration)))
    readonly games!: GameRegistration[]
    constructor(fields: Fields<RegistrationAccount>) {
        Object.assign(this, fields)
    }
}

export class PlayerProfile {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly nick!: string
    @field(option('string'))
    readonly pfp: string | undefined
    constructor(fields: Fields<PlayerProfile>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.PlayerProfile {
        return this
    }
}

export class RecipientSlotShare {
    @field(enums(RecipientSlotOwner))
    owner!: RecipientSlotOwner
    @field('u16')
    weights!: number
    @field('u64')
    claimAmount!: bigint
    constructor(fields: Fields<RecipientSlotShare>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.RecipientSlotShare {
        return {
            owner: this.owner.generalize(),
            weights: this.weights,
            claimAmount: this.claimAmount,
        }
    }
}

export class RecipientSlot {
    @field('u8')
    id!: number
    @field('u8')
    slotType!: RecipientSlotType
    @field('string')
    tokenAddr!: string
    @field(array(struct(RecipientSlotShare)))
    shares!: RecipientSlotShare[]
    @field('u64')
    balance!: bigint
    constructor(fields: Fields<RecipientSlot>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.RecipientSlot {
        return {
            id: this.id,
            slotType: RECIPIENT_SLOT_TYPES[this.slotType],
            tokenAddr: this.tokenAddr,
            shares: this.shares.map(s => s.generalize()),
            balance: this.balance,
        }
    }
}

export class RecipientAccount {
    @field('string')
    addr!: string
    @field(option('string'))
    capAddr: string | undefined
    @field(array(struct(RecipientSlot)))
    slots!: RecipientSlot[]
    constructor(fields: Fields<RecipientAccount>) {
        Object.assign(this, fields)
    }
    generalize(): RaceCore.RecipientAccount {
        return {
            addr: this.addr,
            capAddr: this.capAddr,
            slots: this.slots.map(s => s.generalize()),
        }
    }
}
