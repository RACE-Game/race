import { TxState } from './tx-state'
import { array, enums, field, option, struct, variant } from '@race-foundation/borsh'
import { GameEvent } from './events'
import { CheckpointOffChain } from './checkpoint'
import { Message } from './message'
import { Fields } from './types'
import { DepositStatus } from './accounts'

export class BroadcastPlayerJoin {
    @field('string')
    readonly addr!: string
    @field('u16')
    readonly position!: number
    @field('u64')
    readonly accessVersion!: bigint
    @field('string')
    readonly verifyKey!: string
    constructor(fields: Fields<BroadcastPlayerJoin>) {
        Object.assign(this, fields)
    }
}

export class BroadcastServerJoin {
    @field('string')
    readonly addr!: string
    @field('string')
    readonly endpoint!: string
    @field('u64')
    readonly accessVersion!: bigint
    @field('string')
    readonly verifyKey!: string
    constructor(fields: Fields<BroadcastServerJoin>) {
        Object.assign(this, fields)
    }
}

export class BroadcastPlayerDeposit {
    @field('string')
    readonly addr!: string
    @field('u64')
    readonly amount!: bigint
    @field('u64')
    readonly accessVersion!: bigint
    @field('u64')
    readonly settleVersion!: bigint
    @field('u8')
    readonly status!: DepositStatus
    constructor(fields: Fields<BroadcastPlayerDeposit>) {
        Object.assign(this, fields)
    }
}

export type BroadcastFrameKind = 'Invalid' | 'Event' | 'Message' | 'TxState' | 'Sync' | 'Backlogs'

export abstract class BroadcastFrame {
    kind(): BroadcastFrameKind {
        return 'Invalid'
    }
}

@variant(0)
export class BroadcastFrameEvent extends BroadcastFrame {
    @field(enums(GameEvent))
    event!: GameEvent
    @field('u64')
    timestamp!: bigint
    @field('string')
    stateSha!: string
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, BroadcastFrameEvent.prototype)
    }
    kind(): BroadcastFrameKind {
        return 'Event'
    }
}

@variant(1)
export class BroadcastFrameMessage extends BroadcastFrame {
    @field(struct(Message))
    message!: Message
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, BroadcastFrameMessage.prototype)
    }
    kind(): BroadcastFrameKind {
        return 'Message'
    }
}

@variant(2)
export class BroadcastFrameTxState extends BroadcastFrame {
    @field(enums(TxState))
    txState!: TxState
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, BroadcastFrameTxState.prototype)
    }
    kind(): BroadcastFrameKind {
        return 'TxState'
    }
}

@variant(3)
export class BroadcastFrameSync extends BroadcastFrame {
    @field(array(struct(BroadcastPlayerJoin)))
    newPlayers!: BroadcastPlayerJoin[]
    @field(array(struct(BroadcastServerJoin)))
    newServers!: BroadcastServerJoin[]
    @field(array(struct(BroadcastPlayerDeposit)))
    newDeposits!: BroadcastPlayerDeposit[]
    @field('string')
    transactor_addr!: string
    @field('u64')
    accessVersion!: bigint
    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, BroadcastFrameSync.prototype)
    }
    kind(): BroadcastFrameKind {
        return 'Sync'
    }
}

@variant(4)
export class BroadcastFrameBacklogs extends BroadcastFrame {
    @field(option(struct(CheckpointOffChain)))
    checkpointOffChain: CheckpointOffChain | undefined
    @field(array(enums(BroadcastFrame)))
    backlogs!: BroadcastFrame[]
    @field('string')
    stateSha!: string

    constructor(fields: any) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, BroadcastFrameBacklogs.prototype)
    }
    kind(): BroadcastFrameKind {
        return 'Backlogs'
    }
}
