import { CheckpointOnChain } from './checkpoint'
import { IKind, UnionFromValues } from './types'

export const ENTRY_LOCKS = ['Open', 'JoinOnly', 'DepositOnly', 'Closed'] as const
export type EntryLock = UnionFromValues<typeof ENTRY_LOCKS>

export const DEPOSIT_STATUS = ['Pending', 'Rejected', 'Refunded', 'Accepted'] as const
export type DepositStatus = UnionFromValues<typeof DEPOSIT_STATUS>

export interface PlayerJoin {
    readonly addr: string
    readonly position: number
    readonly accessVersion: bigint
    readonly verifyKey: string
}

export interface PlayerDeposit {
    readonly addr: string
    readonly amount: bigint
    readonly accessVersion: bigint
    readonly settleVersion: bigint
    readonly status: DepositStatus
}

export interface ServerJoin {
    readonly addr: string
    readonly endpoint: string
    readonly accessVersion: bigint
    readonly verifyKey: string
}

export interface Bonus {
    readonly identifier: string
    readonly tokenAddr: string
    readonly amount: bigint
}

export const VOTE_TYPES = ['ServerVoteTransactorDropOff', 'ClientVoteTransactorDropOff'] as const
export type VoteType = UnionFromValues<typeof VOTE_TYPES>

export interface Vote {
    readonly voter: string
    readonly votee: string
    readonly voteType: VoteType
}

export interface GameRegistration {
    readonly title: string
    readonly addr: string
    readonly regTime: bigint
    readonly bundleAddr: string
}

export interface PlayerBalance {
    readonly playerId: bigint
    readonly balance: bigint
}

export interface GameAccount {
    readonly addr: string
    readonly title: string
    readonly bundleAddr: string
    readonly tokenAddr: string
    readonly ownerAddr: string
    readonly settleVersion: bigint
    readonly accessVersion: bigint
    readonly players: PlayerJoin[]
    readonly deposits: PlayerDeposit[]
    readonly servers: ServerJoin[]
    readonly transactorAddr: string | undefined
    readonly votes: Vote[]
    readonly unlockTime: bigint | undefined
    readonly maxPlayers: number
    readonly dataLen: number
    readonly data: Uint8Array
    readonly entryType: EntryType
    readonly recipientAddr: string
    readonly checkpointOnChain: CheckpointOnChain | undefined
    readonly entryLock: EntryLock
    readonly bonuses: Bonus[]
    readonly balances: PlayerBalance[]
}

export interface ServerAccount {
    readonly addr: string
    readonly endpoint: string
}

export interface GameBundle {
    readonly addr: string
    readonly uri: string
    readonly name: string
    readonly data: Uint8Array
}

export interface PlayerProfile {
    readonly addr: string
    readonly nick: string
    readonly pfp: string | undefined
}

export interface RegistrationAccount {
    readonly addr: string
    readonly isPrivate: boolean
    readonly size: number
    readonly owner: string | undefined
    readonly games: GameRegistration[]
}

export interface Token {
    readonly addr: string
    readonly icon: string
    readonly name: string
    readonly symbol: string
    readonly decimals: number
}

export class TokenBalance {
    readonly addr!: string
    readonly amount!: bigint
}

export interface Nft {
    readonly addr: string
    readonly image: string
    readonly name: string
    readonly symbol: string
    readonly collection: string | undefined
    readonly metadata: any
}

export interface RecipientAccount {
    readonly addr: string
    readonly capAddr: string | undefined
    readonly slots: RecipientSlot[]
}

export const RECIPIENT_SLOT_TYPES = ['Nft', 'Token'] as const

export type RecipientSlotType = UnionFromValues<typeof RECIPIENT_SLOT_TYPES>

export interface RecipientSlot {
    readonly id: number
    readonly slotType: RecipientSlotType
    readonly tokenAddr: string
    readonly shares: RecipientSlotShare[]
    readonly balance: bigint
}

export interface RecipientSlotShare {
    readonly owner: RecipientSlotOwner
    readonly weights: number
    readonly claimAmount: bigint
}

export type RecipientSlotOwnerKind<T extends 'unassigned' | 'assigned'> = IKind<T>

export type RecipientSlotOwnerUnassigned = {
    readonly identifier: string
} & RecipientSlotOwnerKind<'unassigned'>

export type RecipientSlotOwnerAssigned = {
    readonly addr: string
} & RecipientSlotOwnerKind<'assigned'>

export type RecipientSlotOwner = RecipientSlotOwnerUnassigned | RecipientSlotOwnerAssigned

export type EntryTypeKind<T extends 'cash' | 'ticket' | 'gating' | 'disabled'> = IKind<T>

export type EntryTypeCash = {
    readonly minDeposit: bigint
    readonly maxDeposit: bigint
} & EntryTypeKind<'cash'>

export type EntryTypeTicket = {
    readonly amount: bigint
} & EntryTypeKind<'ticket'>

export type EntryTypeGating = {
    readonly collection: string
} & EntryTypeKind<'gating'>

export type EntryTypeDisabled = {} & EntryTypeKind<'disabled'>

export type EntryType = EntryTypeCash | EntryTypeTicket | EntryTypeGating | EntryTypeDisabled

/**
 * The registration account data with games consolidated.
 */
export interface RegistrationWithGames {
    readonly addr: string
    readonly isPrivate: boolean
    readonly size: number
    readonly owner: string | undefined
    readonly games: GameAccount[]
}

function getEndpointFromGameAccount(gameAccount: GameAccount): string | undefined {
    const { transactorAddr, servers } = gameAccount;

    if (!transactorAddr) {
        return undefined;
    }

    const server = servers.find(s => s.addr === transactorAddr);

    return server ? server.endpoint : undefined;
}
