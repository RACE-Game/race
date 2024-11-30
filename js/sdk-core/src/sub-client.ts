import { BaseClient } from './base-client'
import { Client } from './client'
import { IConnection, SubscribeEventParams } from './connection'
import { DecryptionCache } from './decryption-cache'
import { IEncryptor } from './encryptor'
import { GameContext } from './game-context'
import { Handler } from './handler'
import { ITransport } from './transport'
import {
    GameInfo,
    ConnectionStateCallbackFunction,
    EventCallbackFunction,
    MessageCallbackFunction,
    TxStateCallbackFunction,
    ErrorCallbackFunction,
} from './types'
import { IWallet } from './wallet'
import { CheckpointOnChain } from './checkpoint'

export type SubClientCtorOpts = {
    gameAddr: string
    gameId: number
    handler: Handler
    wallet: IWallet
    client: Client
    transport: ITransport
    connection: IConnection
    gameContext: GameContext
    latestCheckpointOnChain: CheckpointOnChain | undefined
    onEvent: EventCallbackFunction
    onMessage: MessageCallbackFunction | undefined
    onTxState: TxStateCallbackFunction | undefined
    onConnectionState: ConnectionStateCallbackFunction | undefined
    onError: ErrorCallbackFunction | undefined
    encryptor: IEncryptor
    info: GameInfo
    decryptionCache: DecryptionCache
}

export class SubClient extends BaseClient {
    constructor(opts: SubClientCtorOpts) {
        super({
            onLoadProfile: (_id: bigint, _addr: string) => {},
            logPrefix: `SubGame#${opts.gameId}|`,
            ...opts,
        })
    }

    /**
     * Connect to the transactor and retrieve the event stream.
     */
    async attachGame() {
        console.group(`${this.__logPrefix}Attach to game`)
        let sub
        try {
            await this.__attachGameWithRetry()
            sub = this.__connection.subscribeEvents()
            const settleVersion = this.__gameContext.checkpointVersion() || 0n
            await this.__connection.connect(new SubscribeEventParams({ settleVersion }))
        } catch (e) {
            console.error('Attaching game failed', e)
            throw e
        } finally {
            console.groupEnd()
        }
        await this.__processSubscription(sub)
    }
}
