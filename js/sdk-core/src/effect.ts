import { RandomSpec } from './random-state'
import { HandleError } from './error'
import { GameContext } from './game-context'
import { enums, field, map, option, struct, array } from '@race-foundation/borsh'
import { Fields, Indices } from './types'
import { InitAccount } from './init-account'
import { EntryLock } from './accounts'

export class Settle {
    @field('u64')
    id: bigint
    @field('u64')
    amount: bigint
    @field('bool')
    eject: boolean
    constructor(fields: Fields<Settle>) {
        this.id = fields.id
        this.amount = fields.amount
        this.eject = fields.eject
    }
}

export class Transfer {
    @field('u8')
    slotId!: number
    @field('u64')
    amount!: bigint
    constructor(fields: Fields<Transfer>) {
        Object.assign(this, fields)
    }
}

export class Award {
    @field('u64')
    id: bigint
    @field('string')
    bonusIdentifier: string
    constructor(fields: Fields<Award>) {
        this.id = fields.id
        this.bonusIdentifier = fields.bonusIdentifier
    }
}

export class Ask {
    @field('string')
    playerAddr!: string
    constructor(fields: Fields<Ask>) {
        Object.assign(this, fields)
    }
}

export class Assign {
    @field('usize')
    randomId!: number
    @field('u64')
    playerId!: bigint
    @field(array('usize'))
    indexes!: number[]
    constructor(fields: Fields<Assign>) {
        Object.assign(this, fields)
    }
}

export class Reveal {
    @field('usize')
    randomId!: number
    @field(array('usize'))
    indexes!: number[]
    constructor(fields: Fields<Reveal>) {
        Object.assign(this, fields)
    }
}

export class Release {
    @field('usize')
    decisionId!: number
    constructor(fields: Fields<Release>) {
        Object.assign(this, fields)
    }
}

export class ActionTimeout {
    @field('u64')
    playerId!: bigint
    @field('u64')
    timeout!: bigint
    constructor(fields: Fields<ActionTimeout>) {
        Object.assign(this, fields)
    }
}

export class SubGame {
    @field('usize')
    gameId!: number
    @field('string')
    bundleAddr!: string
    @field(struct(InitAccount))
    initAccount!: InitAccount
    constructor(fields: Fields<SubGame>) {
        Object.assign(this, fields)
    }
}

export class EmitBridgeEvent {
    @field('usize')
    dest!: number
    @field('u8-array')
    raw!: Uint8Array

    constructor(fields: Fields<EmitBridgeEvent>) {
        Object.assign(this, fields)
    }
}

export const LOG_LEVELS = ['debug', 'info', 'warn', 'error'] as const
export type LogLevel = Indices<typeof LOG_LEVELS>

export class Log {
    @field('u8')
    level!: LogLevel

    @field('string')
    message!: string

    constructor(fields: Fields<Log>) {
        Object.assign(this, fields)
    }
}

export class Effect {
    @field(option(struct(ActionTimeout)))
    actionTimeout: ActionTimeout | undefined
    @field(option('u64'))
    waitTimeout: bigint | undefined
    @field('bool')
    startGame!: boolean
    @field('bool')
    stopGame!: boolean
    @field('bool')
    cancelDispatch!: boolean
    @field('u64')
    timestamp!: bigint
    @field('usize')
    currRandomId!: number
    @field('usize')
    currDecisionId!: number
    @field('u16')
    nodesCount!: number
    @field(array(struct(Ask)))
    asks!: Ask[]
    @field(array(struct(Assign)))
    assigns!: Assign[]
    @field(array(struct(Reveal)))
    reveals!: Reveal[]
    @field(array(struct(Release)))
    releases!: Release[]
    @field(array(enums(RandomSpec)))
    initRandomStates!: RandomSpec[]
    @field(map('usize', map('usize', 'string')))
    revealed!: Map<number, Map<number, string>>
    @field(map('usize', 'string'))
    answered!: Map<number, string>
    @field('bool')
    isCheckpoint!: boolean
    @field(array(struct(Settle)))
    settles!: Settle[]
    @field(option('u8-array'))
    handlerState!: Uint8Array | undefined
    @field(option(enums(HandleError)))
    error: HandleError | undefined
    @field(array(struct(Transfer)))
    transfers!: Transfer[]
    @field(array(struct(SubGame)))
    launchSubGames!: SubGame[]
    @field(array(struct(EmitBridgeEvent)))
    bridgeEvents!: EmitBridgeEvent[]
    @field(array('u64'))
    validPlayers!: bigint[]
    @field('bool')
    isInit!: boolean
    @field(option('u8'))
    entryLock!: EntryLock | undefined
    @field('bool')
    reset!: boolean
    @field(array(struct(Log)))
    logs!: Log[]
    @field(array(struct(Award)))
    awards!: Award[]
    @field(array('u64'))
    rejectDeposits!: bigint[]
    @field(array('u64'))
    acceptDeposits!: bigint[]
    @field('usize')
    currSubGameId!: number

    constructor(fields: Fields<Effect>) {
        Object.assign(this, fields)
    }

    static fromContext(context: GameContext, isInit: boolean) {
        const revealed = new Map<number, Map<number, string>>()
        for (const st of context.randomStates) {
            revealed.set(st.id, st.revealed)
        }
        const answered = new Map<number, string>()
        for (const st of context.decisionStates) {
            answered.set(st.id, st.value!)
        }
        const actionTimeout = undefined
        const waitTimeout = undefined
        const startGame = false
        const stopGame = false
        const cancelDispatch = false
        const timestamp = isInit ? 0n : context.timestamp
        const currRandomId = context.randomStates.length + 1
        const currDecisionId = context.decisionStates.length + 1
        const nodesCount = context.nodes.length
        const asks: Ask[] = []
        const assigns: Assign[] = []
        const releases: Release[] = []
        const reveals: Reveal[] = []
        const initRandomStates: RandomSpec[] = []
        const isCheckpoint = false
        const settles: Settle[] = []
        const handlerState = context.handlerState
        const error = undefined
        const transfers: Transfer[] = []
        const launchSubGames: SubGame[] = []
        const bridgeEvents: EmitBridgeEvent[] = []
        const validPlayers = context.players.map(p => p.id)
        const entryLock = undefined
        const reset = false
        const logs: Log[] = []
        const awards: Award[] = []
        const rejectDeposits: bigint[] = []
        const acceptDeposits: bigint[] = []
        const currSubGameId = context.subGames.length + 1
        return new Effect({
            actionTimeout,
            waitTimeout,
            startGame,
            stopGame,
            cancelDispatch,
            timestamp,
            currRandomId,
            currDecisionId,
            nodesCount,
            asks,
            assigns,
            releases,
            reveals,
            initRandomStates,
            revealed,
            answered,
            isCheckpoint,
            settles,
            handlerState,
            error,
            transfers,
            launchSubGames,
            bridgeEvents,
            validPlayers,
            isInit,
            entryLock,
            reset,
            logs,
            awards,
            rejectDeposits,
            acceptDeposits,
            currSubGameId,
        })
    }
}
