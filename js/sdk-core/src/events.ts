import { field, array, enums, option, variant, struct } from '@race-foundation/borsh'
import { Fields } from './types'

export type EventKind =
    | 'Invalid' // an invalid value
    | 'Custom'
    | 'Ready'
    | 'ShareSecrets'
    | 'OperationTimeout'
    | 'Mask'
    | 'Lock'
    | 'RandomnessReady'
    | 'Join'
    | 'Deposit'
    | 'ServerLeave'
    | 'Leave'
    | 'GameStart'
    | 'WaitingTimeout'
    | 'DrawRandomItems'
    | 'DrawTimeout'
    | 'ActionTimeout'
    | 'AnswerDecision'
    | 'SecretsReady'
    | 'Shutdown'
    | 'Bridge'
    | 'SubGameReady'

export interface ICustomEvent {
    serialize(): Uint8Array
}

export interface IBridgeEvent {
    serialize(): Uint8Array
}

interface IEventKind {
    kind(): EventKind
}

export class GamePlayer {
    @field('u64')
    id!: bigint
    @field('u16')
    position!: number
    constructor(fields: Fields<GamePlayer>) {
        Object.assign(this, fields)
    }
}

export class GameDeposit {
    @field('u64')
    id!: bigint
    @field('u64')
    amount!: bigint
    @field('u64')
    accessVersion!: bigint
    constructor(fields: Fields<GameDeposit>) {
        Object.assign(this, fields)
    }
}

export abstract class SecretShare {}

@variant(0)
export class Random extends SecretShare {
    @field('string')
    fromAddr!: string
    @field(option('string'))
    toAddr!: string | undefined
    @field('usize')
    randomId!: number
    @field('usize')
    index!: number
    @field('u8-array')
    secret!: Uint8Array
    constructor(fields: Fields<Random>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Random.prototype)
    }
}

@variant(1)
export class Answer extends SecretShare {
    @field('string')
    fromAddr!: string
    @field('usize')
    decisionId!: number
    @field('u8-array')
    secret!: Uint8Array
    constructor(fields: Fields<Answer>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Answer.prototype)
    }
}

export abstract class GameEvent implements IEventKind {
    kind(): EventKind {
        return 'Invalid'
    }
}

@variant(0)
export class Custom extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field('u8-array')
    raw!: Uint8Array
    constructor(fields: Fields<Custom>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Custom.prototype)
    }
    kind(): EventKind {
        return 'Custom'
    }
}

export function makeCustomEvent(sender: bigint, customEvent: ICustomEvent): Custom {
    return new Custom({
        sender,
        raw: customEvent.serialize(),
    })
}

@variant(1)
export class Ready extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super()
        Object.setPrototypeOf(this, Ready.prototype)
    }
    kind(): EventKind {
        return 'Ready'
    }
}

@variant(2)
export class ShareSecrets extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field(array(enums(SecretShare)))
    shares!: SecretShare[]
    constructor(fields: Fields<ShareSecrets>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, ShareSecrets.prototype)
    }
    kind(): EventKind {
        return 'ShareSecrets'
    }
}

@variant(3)
export class OperationTimeout extends GameEvent implements IEventKind {
    @field(array('u64'))
    ids!: bigint[]
    constructor(fields: Fields<OperationTimeout>) {
        super()
        Object.setPrototypeOf(this, OperationTimeout.prototype)
        Object.assign(this, fields)
    }
    kind(): EventKind {
        return 'OperationTimeout'
    }
}

@variant(4)
export class Mask extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field('usize')
    randomId!: number
    @field(array('u8-array'))
    ciphertexts!: Uint8Array[]
    constructor(fields: Fields<Mask>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Mask.prototype)
    }
    kind(): EventKind {
        return 'Mask'
    }
}

export class CiphertextAndDigest {
    @field('u8-array')
    ciphertext!: Uint8Array
    @field('u8-array')
    digest!: Uint8Array
    constructor(fields: Fields<CiphertextAndDigest>) {
        Object.assign(this, fields)
    }
}

@variant(5)
export class Lock extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field('usize')
    randomId!: number
    @field(array(struct(CiphertextAndDigest)))
    ciphertextsAndDigests!: CiphertextAndDigest[]
    constructor(fields: Fields<Lock>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Lock.prototype)
    }
    kind(): EventKind {
        return 'Lock'
    }
}

@variant(6)
export class RandomnessReady extends GameEvent implements IEventKind {
    @field('usize')
    randomId!: number
    constructor(fields: Fields<RandomnessReady>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, RandomnessReady.prototype)
    }
    kind(): EventKind {
        return 'RandomnessReady'
    }
}

@variant(7)
export class Join extends GameEvent implements IEventKind {
    @field(array(struct(GamePlayer)))
    players!: GamePlayer[]
    constructor(fields: Fields<Join>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Join.prototype)
    }
    kind(): EventKind {
        return 'Join'
    }
}

@variant(8)
export class Deposit extends GameEvent implements IEventKind {
    @field(array(struct(GameDeposit)))
    deposits!: GameDeposit[]
    constructor(fields: Fields<Deposit>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Deposit.prototype)
    }
    kind(): EventKind {
        return 'Deposit'
    }
}

@variant(9)
export class ServerLeave extends GameEvent implements IEventKind {
    @field('u64')
    serverId!: bigint
    constructor(fields: Fields<ServerLeave>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, ServerLeave.prototype)
    }
    kind(): EventKind {
        return 'ServerLeave'
    }
}

@variant(10)
export class Leave extends GameEvent implements IEventKind {
    @field('u64')
    playerId!: bigint
    constructor(fields: Fields<Leave>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Leave.prototype)
    }
    kind(): EventKind {
        return 'Leave'
    }
}

@variant(11)
export class GameStart extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super()
        Object.setPrototypeOf(this, GameStart.prototype)
    }
    kind(): EventKind {
        return 'GameStart'
    }
}

@variant(12)
export class WaitingTimeout extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super()
        Object.setPrototypeOf(this, WaitingTimeout.prototype)
    }
    kind(): EventKind {
        return 'WaitingTimeout'
    }
}

@variant(13)
export class DrawRandomItems extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field('usize')
    randomId!: number
    @field(array('usize'))
    indexes!: number[]
    constructor(fields: Fields<DrawRandomItems>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, DrawRandomItems.prototype)
    }
    kind(): EventKind {
        return 'DrawRandomItems'
    }
}

@variant(14)
export class DrawTimeout extends GameEvent implements IEventKind {
    constructor(_: {}) {
        super()
        Object.setPrototypeOf(this, DrawTimeout.prototype)
    }
    kind(): EventKind {
        return 'DrawTimeout'
    }
}

@variant(15)
export class ActionTimeout extends GameEvent implements IEventKind {
    @field('u64')
    playerId!: bigint
    constructor(fields: Fields<ActionTimeout>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, ActionTimeout.prototype)
    }
    kind(): EventKind {
        return 'ActionTimeout'
    }
}

@variant(16)
export class AnswerDecision extends GameEvent implements IEventKind {
    @field('u64')
    sender!: bigint
    @field('usize')
    decisionId!: number
    @field('u8-array')
    ciphertext!: Uint8Array
    @field('u8-array')
    digest!: Uint8Array
    constructor(fields: Fields<AnswerDecision>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, AnswerDecision.prototype)
    }
    kind(): EventKind {
        return 'AnswerDecision'
    }
}

@variant(17)
export class SecretsReady extends GameEvent implements IEventKind {
    @field(array('usize'))
    randomIds!: number[]

    constructor(fields: Fields<SecretsReady>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, SecretsReady.prototype)
    }
    kind(): EventKind {
        return 'SecretsReady'
    }
}

@variant(18)
export class Shutdown extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super()
        Object.setPrototypeOf(this, Shutdown.prototype)
    }
    kind(): EventKind {
        return 'Shutdown'
    }
}

@variant(19)
export class Bridge extends GameEvent implements IEventKind {
    @field('usize')
    destGameId!: number
    @field('usize')
    fromGameId!: number
    @field('u8-array')
    raw!: Uint8Array

    constructor(fields: Fields<Bridge>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, Bridge.prototype)
    }

    kind(): EventKind {
        return 'Bridge'
    }
}

@variant(20)
export class SubGameReady extends GameEvent implements IEventKind {
    @field('usize')
    gameId!: number
    @field('u16')
    maxPlayers!: number
    @field('u8-array')
    initData!: Uint8Array

    constructor(fields: Fields<SubGameReady>) {
        super()
        Object.assign(this, fields)
        Object.setPrototypeOf(this, SubGameReady.prototype)
    }

    kind(): EventKind {
        return 'SubGameReady'
    }
}
