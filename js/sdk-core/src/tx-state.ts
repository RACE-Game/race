import { field, array, variant, struct, option } from '@race-foundation/borsh'

export abstract class TxState {}

export type TxStateKind = 'PlayerConfirming' | 'PlayerConfirmingFailed' | 'SettleSucceed'

export interface ITxStateKind {
    kind(): TxStateKind
}

export class ConfirmingPlayer {
    @field('u64')
    id!: bigint
    @field('string')
    addr!: string
    @field('u16')
    position!: number
    @field('u64')
    balance!: bigint

    constructor(fields: any) {
        Object.assign(this, fields)
    }
}

@variant(0)
export class PlayerConfirming extends TxState implements ITxStateKind {
    @field(array(struct(ConfirmingPlayer)))
    confirmPlayers!: ConfirmingPlayer[]
    @field('u64')
    accessVersion!: bigint

    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
    kind(): TxStateKind {
        return 'PlayerConfirming'
    }
}

@variant(1)
export class PlayerConfirmingFailed extends TxState implements ITxStateKind {
    @field('u64')
    accessVersion!: bigint

    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
    kind(): TxStateKind {
        return 'PlayerConfirmingFailed'
    }
}

@variant(2)
export class SettleSucceed extends TxState implements ITxStateKind {
    @field('u64')
    settleVersion!: bigint
    @field(option('string'))
    signature: string | undefined

    constructor(fields: any) {
        super()
        Object.assign(this, fields)
    }
    kind(): TxStateKind {
        return 'SettleSucceed'
    }
}
