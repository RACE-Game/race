import { EntryType, Nft, Token } from './accounts'
import { Message } from './message'
import { ConnectionState } from './connection'
import { GameEvent } from './events'
import { GameContextSnapshot } from './game-context-snapshot'
import { TxState } from './tx-state'

export type Ciphertext = Uint8Array

export type Secret = Uint8Array

export type Digest = Uint8Array

export type Fields<T> = { [K in keyof T as T[K] extends Function ? never : K]: T[K] }

export type Result<T, E> = { ok: T } | { err: E }

export type IKind<T> = { kind: T }

export type Indices<T extends readonly any[]> = Exclude<Partial<T>['length'], T['length']>

export type UnionFromValues<T> = T extends readonly string[] ? T[number] : never

export type GameInfo = {
    gameAddr: string
    title: string
    maxPlayers: number
    entryType: EntryType
    token: Token
    tokenAddr: string
    bundleAddr: string
    data: Uint8Array
    dataLen: number
}

export type PlayerProfileWithPfp = {
    pfp: Nft | undefined
    addr: string
    nick: string
}

export type EventCallbackOptions = {
    isCheckpoint: boolean
}

export type EventCallbackFunction = (
    context: GameContextSnapshot,
    state: Uint8Array,
    event: GameEvent,
    options: EventCallbackOptions
) => void

export type ErrorKind =
    | 'event-state-sha-mismatch'
    | 'checkpoint-state-sha-mismatch'
    | 'onchain-data-not-found'
    | 'attach-failed'
    | 'handle-event-error'
    | 'init-data-invalid'

export type MessageCallbackFunction = (message: Message) => void

export type TxStateCallbackFunction = (txState: TxState) => void

export type ConnectionStateCallbackFunction = (connState: ConnectionState) => void

export type ProfileCallbackFunction = (id: bigint | undefined, profile: PlayerProfileWithPfp) => void

export type LoadProfileCallbackFunction = (id: bigint, addr: string) => void

export type ErrorCallbackFunction = (error: ErrorKind, arg: any) => void

export type ReadyCallbackFunction = (context: GameContextSnapshot, state: Uint8Array) => void
