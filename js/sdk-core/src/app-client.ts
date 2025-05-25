import { Connection, GetCheckpointParams, IConnection } from './connection'
import { GameContext } from './game-context'
import { DepositError, DepositResponse, ITransport, JoinError, JoinResponse } from './transport'
import { IWallet } from './wallet'
import { Handler } from './handler'
import { Encryptor, IEncryptor, sha256String } from './encryptor'
import { SdkError } from './error'
import { Client } from './client'
import { IStorage, getTtlCache, makeBundleCacheKey, setTtlCache } from './storage'
import { DecryptionCache } from './decryption-cache'
import { ProfileLoader } from './profile-loader'
import { BaseClient } from './base-client'
import { GameAccount, GameBundle, Token } from './accounts'
import {
    ConnectionStateCallbackFunction,
    EventCallbackFunction,
    GameInfo,
    MessageCallbackFunction,
    TxStateCallbackFunction,
    PlayerProfileWithPfp,
    ProfileCallbackFunction,
    ErrorCallbackFunction,
    ReadyCallbackFunction,
} from './types'
import { SubClient } from './sub-client'
import { Checkpoint, CheckpointOffChain } from './checkpoint'
import { ResponseHandle, ResponseStream } from './response'
import { BUNDLE_CACHE_TTL } from './common'

export type AppClientInitOpts = {
    transport: ITransport
    wallet: IWallet
    gameAddr: string
    onProfile?: ProfileCallbackFunction
    onEvent: EventCallbackFunction
    onMessage?: MessageCallbackFunction
    onTxState?: TxStateCallbackFunction
    onError?: ErrorCallbackFunction
    onReady?: ReadyCallbackFunction
    onConnectionState?: ConnectionStateCallbackFunction
    storage?: IStorage
    maxRetries?: number
}

export type SubClientInitOpts = {
    gameId: number
    gameAddr: string
    onEvent: EventCallbackFunction
    onMessage?: MessageCallbackFunction
    onTxState?: TxStateCallbackFunction
    onError?: ErrorCallbackFunction
    onConnectionState?: ConnectionStateCallbackFunction
    onReady?: ReadyCallbackFunction
}

export type JoinOpts = {
    amount: bigint
    position?: number
    createProfileIfNeeded?: boolean
}

export type DepositOpts = {
    amount: bigint
}

export type AppClientCtorOpts = {
    gameAddr: string
    gameAccount: GameAccount
    handler: Handler
    wallet: IWallet
    client: Client
    transport: ITransport
    connection: IConnection
    gameContext: GameContext
    onEvent: EventCallbackFunction
    onMessage: MessageCallbackFunction | undefined
    onTxState: TxStateCallbackFunction | undefined
    onConnectionState: ConnectionStateCallbackFunction | undefined
    onError: ErrorCallbackFunction | undefined
    onReady: ReadyCallbackFunction | undefined
    encryptor: IEncryptor
    info: GameInfo
    decryptionCache: DecryptionCache
    profileLoader: ProfileLoader
    storage: IStorage | undefined
    endpoint: string
    maxRetries: number
}

export class AppClient extends BaseClient {
    __profileLoader: ProfileLoader
    __storage?: IStorage
    __endpoint: string
    __latestGameAccount: GameAccount

    constructor(opts: AppClientCtorOpts) {
        super({
            onLoadProfile: (id, addr) => opts.profileLoader.load(id, addr),
            logPrefix: 'MainGame|',
            gameId: 0,
            latestCheckpointOnChain: opts.gameAccount.checkpointOnChain,
            ...opts,
        })
        this.__profileLoader = opts.profileLoader
        this.__storage = opts.storage
        this.__endpoint = opts.endpoint
        this.__latestGameAccount = opts.gameAccount
    }

    static async initialize(opts: AppClientInitOpts): Promise<AppClient> {
        const {
            transport,
            wallet,
            gameAddr,
            onEvent,
            onMessage,
            onTxState,
            onConnectionState,
            onError,
            onProfile,
            onReady,
            storage,
            maxRetries,
        } = opts

        const _maxRetries = maxRetries === undefined ? 10 : maxRetries

        console.group(`Initialize AppClient, gameAddr = ${gameAddr}`)
        try {
            let startTime = new Date().getTime()
            const playerAddr = wallet.walletAddr
            console.info(`Player address: ${playerAddr}`)
            const gameAccount = await transport.getGameAccount(gameAddr)
            console.info('Game Account:', gameAccount)
            if (gameAccount === undefined) {
                throw SdkError.gameAccountNotFound(gameAddr)
            }

            const transactorAddr = gameAccount.transactorAddr
            console.info(`Transactor address: ${transactorAddr}`)
            if (transactorAddr === undefined || gameAccount.checkpointOnChain === undefined) {
                throw SdkError.gameNotServed(gameAddr)
            }

            const bundleCacheKey = makeBundleCacheKey(transport.chain, gameAccount.bundleAddr)
            let token: Token | undefined = await transport.getToken(gameAccount.tokenAddr)

            const [encryptor, gameBundle, transactorAccount] = await Promise.all([
                Encryptor.create(playerAddr, storage),
                getGameBundle(transport, storage, bundleCacheKey, gameAccount.bundleAddr),
                transport.getServerAccount(transactorAddr),
                transport.getToken(gameAccount.tokenAddr),
            ])

            if (transactorAddr === undefined || gameAccount.checkpointOnChain === undefined) {
                throw SdkError.gameNotServed(gameAddr)
            }
            if (transactorAccount === undefined) {
                throw SdkError.transactorAccountNotFound(transactorAddr)
            }
            const decryptionCache = new DecryptionCache()
            const endpoint = transactorAccount.endpoint
            const connection = Connection.initialize(gameAddr, playerAddr, endpoint, encryptor)
            console.info(`Connected with transactor: ${endpoint}`)
            const client = new Client(playerAddr, encryptor, connection)
            console.info(`Client created`)

            const getCheckpointParams: GetCheckpointParams = new GetCheckpointParams({
                settleVersion: gameAccount.settleVersion,
            })

            console.info('Initialize wasm handler and fetch checkpoint')
            const [handler, checkpointOffChain] = await Promise.all([
                Handler.initialize(gameBundle, encryptor, client, decryptionCache),
                await connection.getCheckpoint(getCheckpointParams),
            ])

            if (gameAccount.checkpointOnChain !== undefined) {
                if (checkpointOffChain === undefined) {
                    throw new Error('No checkpoint from transactor.')
                }
            }

            console.info('The onchain part of checkpoint:', gameAccount.checkpointOnChain)
            console.info('The offchain part of checkpoint:', checkpointOffChain)
            let checkpoint
            if (checkpointOffChain !== undefined && gameAccount.checkpointOnChain !== undefined) {
                checkpoint = Checkpoint.fromParts(checkpointOffChain, gameAccount.checkpointOnChain)
            } else {
                throw SdkError.gameNotServed(gameAddr)
            }

            const gameContext = new GameContext(gameAccount, checkpoint)

            if (token === undefined) {
                const decimals = await transport.getTokenDecimals(gameAccount.tokenAddr)
                if (decimals === undefined) {
                    throw SdkError.tokenNotFound(gameAccount.tokenAddr)
                } else {
                    token = {
                        addr: gameAccount.tokenAddr,
                        decimals: decimals,
                        icon: '',
                        name: '-',
                        symbol: '-',
                    }
                }
            }
            const info = makeGameInfo(gameAccount, token)
            const profileLoader = new ProfileLoader(transport, storage, onProfile)
            profileLoader.start()

            const cost = new Date().getTime() - startTime;
            console.info(`Initialize cost ${cost} ms`)

            return new AppClient({
                gameAddr,
                gameAccount,
                handler,
                wallet,
                client,
                transport,
                connection,
                gameContext,
                onEvent,
                onMessage,
                onTxState,
                onConnectionState,
                onError,
                onReady,
                encryptor,
                info,
                decryptionCache,
                profileLoader,
                storage,
                endpoint,
                maxRetries: _maxRetries
            })
        } finally {
            console.groupEnd()
        }
    }

    /**
     * Create a client for subgame.
     *
     *
     */
    async subClient(opts: SubClientInitOpts): Promise<SubClient> {
        try {
            const { gameId, onEvent, onMessage, onTxState, onConnectionState, onError, onReady } = opts

            const addr = `${this.__gameAddr}:${gameId.toString()}`

            console.group(`SubClient initialization, id: ${gameId}`)
            console.info('Versioned data:', this.__gameContext.checkpoint.getVersionedData(gameId))

            const subGame = this.__gameContext.findSubGame(gameId)

            if (subGame === undefined) {
                console.warn('Game context:', this.__gameContext)
                throw SdkError.invalidSubId(gameId)
            } else {
                console.info('Sub Game:', subGame)
            }

            const bundleAddr = subGame.bundleAddr

            const bundleCacheKey = makeBundleCacheKey(this.__transport.chain, bundleAddr)
            const decryptionCache = new DecryptionCache()
            const playerAddr = this.__wallet.walletAddr
            const connection = Connection.initialize(addr, playerAddr, this.__endpoint, this.__encryptor)
            const client = new Client(playerAddr, this.__encryptor, connection)
            const gameBundle = await getGameBundle(this.__transport, this.__storage, bundleCacheKey, bundleAddr)
            const handler = await Handler.initialize(gameBundle, this.__encryptor, client, decryptionCache)

            const gameContext = this.__gameContext.subContext(subGame)

            return new SubClient({
                gameAddr: addr,
                wallet: this.__wallet,
                transport: this.__transport,
                encryptor: this.__encryptor,
                onEvent,
                onMessage,
                onTxState,
                onConnectionState,
                onError,
                onReady,
                handler,
                connection,
                client,
                info: this.__info,
                decryptionCache,
                gameContext,
                gameId,
                latestCheckpointOnChain: undefined,
                maxRetries: this.__maxRetries,
            })
        } finally {
            console.groupEnd()
        }
    }

    /**
     * Connect to the transactor and retrieve the event stream.
     */
    async attachGame() {
        let sub
        try {
            await this.__attachGameWithRetry()
            this.__startSubscribe()
            for (const p of this.__latestGameAccount.players) {
                console.log('Load profile for', p.addr)
                this.__onLoadProfile(p.accessVersion, p.addr)
            }
        } catch (e) {
            console.error(this.__logPrefix + 'Attaching game failed', e)
            this.__invokeErrorCallback('attach-failed')
            throw e
        } finally {
            console.groupEnd()
        }
        await this.__processSubscription()
    }

    /**
     * Get player profile by its wallet address.
     */
    getProfile(id: bigint): PlayerProfileWithPfp | undefined
    getProfile(addr: string): PlayerProfileWithPfp | undefined
    getProfile(idOrAddr: string | bigint): PlayerProfileWithPfp | undefined {
        let addr: string = ''
        try {
            if (typeof idOrAddr === 'bigint') {
                addr = this.__gameContext.idToAddr(idOrAddr)
            } else {
                addr = idOrAddr
            }
        } catch (e) {
            return undefined
        }
        return this.__profileLoader.getProfile(addr)
    }

    /**
     * Return if current player is in game.
     */
    isInGame(): boolean {
        try {
            const playerId = this.addrToId(this.__wallet.walletAddr)
            if (this.__gameContext.players.find(p => p.id === playerId) !== undefined) {
                return true
            }
            return false
        } catch (e) {
            return false
        }
    }

    makeSubGameAddr(gameId: number): string {
        return `${this.__gameAddr}:${gameId}`
    }

    /**
     * Initiates a join request for a game session. It exports a public key and
     * sends a join request with required parameters like game address, amount,
     * position, and whether to create a profile if needed. Returns a stream to
     * handle the response of the join operation, which can either be a success
     * (JoinResponse) or an error (JoinError).
     *
     * @param {JoinOpts} params - Options and parameters to configure the join request.
     * @returns {ResponseStream<JoinResponse, JoinError>} A stream to handle the
     * response of the join request.
     */
    join(params: JoinOpts): ResponseStream<JoinResponse, JoinError> {
        const response = new ResponseHandle<JoinResponse, JoinError>()

        this.__encryptor.exportPublicKey().then(publicKey => {
            this.__transport.join(
                this.__wallet,
                {
                    gameAddr: this.gameAddr,
                    amount: params.amount,
                    position: params.position || 0,
                    verifyKey: publicKey.ec,
                    createProfileIfNeeded: params.createProfileIfNeeded,
                },
                response
            )
        })

        return response.stream()
    }

    deposit(params: DepositOpts): ResponseStream<DepositResponse, DepositError> {
        const response = new ResponseHandle<DepositResponse, DepositError>()

        this.__getGameAccount().then(gameAccount => {
            this.__transport.deposit(
                this.__wallet,
                {
                    gameAddr: this.gameAddr,
                    amount: params.amount,
                    settleVersion: gameAccount.settleVersion,
                },
                response
            )
        })

        return response.stream()
    }
}

// Miscellaneous

export async function getGameBundle(
    transport: ITransport,
    storage: IStorage | undefined,
    bundleCacheKey: string,
    bundleAddr: string
): Promise<GameBundle> {
    let gameBundle: GameBundle | undefined
    if (storage !== undefined) {
        gameBundle = getTtlCache(storage, bundleCacheKey)
        console.debug('Use game bundle from cache:', gameBundle)
        if (gameBundle !== undefined) {
            Object.assign(gameBundle, { data: Uint8Array.of() })
        }
    }
    if (gameBundle === undefined) {
        gameBundle = await transport.getGameBundle(bundleAddr)
        console.debug('Game bundle:', gameBundle)
        if (gameBundle !== undefined && storage !== undefined && gameBundle.data.length === 0) {
            setTtlCache(storage, bundleCacheKey, gameBundle, BUNDLE_CACHE_TTL)
        }
    }
    if (gameBundle === undefined) {
        throw SdkError.gameBundleNotFound(bundleAddr)
    }
    return gameBundle
}

export function makeGameInfo(gameAccount: GameAccount, token: Token): GameInfo {
    const info: GameInfo = {
        gameAddr: gameAccount.addr,
        title: gameAccount.title,
        entryType: gameAccount.entryType,
        maxPlayers: gameAccount.maxPlayers,
        tokenAddr: gameAccount.tokenAddr,
        bundleAddr: gameAccount.bundleAddr,
        data: gameAccount.data,
        dataLen: gameAccount.dataLen,
        token,
    }

    return info
}
