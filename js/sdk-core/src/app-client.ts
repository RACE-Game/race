import { Connection, GetCheckpointParams, IConnection } from './connection'
import { GameContext } from './game-context'
import { ITransport, JoinError, JoinResponse } from './transport'
import { IWallet } from './wallet'
import { Handler } from './handler'
import { Encryptor, IEncryptor } from './encryptor'
import { SdkError } from './error'
import { Client } from './client'
import { IStorage, getTtlCache, makeBundleCacheKey, setTtlCache } from './storage'
import { DecryptionCache } from './decryption-cache'
import { ProfileLoader } from './profile-loader'
import { BaseClient } from './base-client'
import { EntryTypeCash, GameAccount, GameBundle, IToken } from './accounts'
import {
  ConnectionStateCallbackFunction,
  EventCallbackFunction,
  GameInfo,
  MessageCallbackFunction,
  TxStateCallbackFunction,
  PlayerProfileWithPfp,
  ProfileCallbackFunction,
  ErrorCallbackFunction,
} from './types'
import { SubClient } from './sub-client'
import { Checkpoint, CheckpointOffChain } from './checkpoint'
import { clone } from './utils'
import { ResponseHandle, ResponseStream } from './response'

const BUNDLE_CACHE_TTL = 3600 * 365

export type AppClientInitOpts = {
  transport: ITransport
  wallet: IWallet
  gameAddr: string
  onProfile?: ProfileCallbackFunction
  onEvent: EventCallbackFunction
  onMessage?: MessageCallbackFunction
  onTxState?: TxStateCallbackFunction
  onError?: ErrorCallbackFunction
  onConnectionState?: ConnectionStateCallbackFunction
  storage?: IStorage
}

export type SubClientInitOpts = {
  gameId: number
  gameAddr: string
  onEvent: EventCallbackFunction
  onMessage?: MessageCallbackFunction
  onTxState?: TxStateCallbackFunction
  onError?: ErrorCallbackFunction
  onConnectionState?: ConnectionStateCallbackFunction
}

export type JoinOpts = {
  amount: bigint
  position?: number
  createProfileIfNeeded?: boolean
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
  encryptor: IEncryptor
  info: GameInfo
  decryptionCache: DecryptionCache
  profileLoader: ProfileLoader
  storage: IStorage | undefined
  endpoint: string
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
      storage,
    } = opts

    console.group(`Initialize AppClient, gameAddr = ${gameAddr}`)
    try {
      const playerAddr = wallet.walletAddr
      console.log(`PlayerAddr = ${playerAddr}`)
      const encryptor = await Encryptor.create(playerAddr, storage)
      const gameAccount = await transport.getGameAccount(gameAddr)
      console.log('Game Account:', gameAccount)
      if (gameAccount === undefined) {
        throw SdkError.gameAccountNotFound(gameAddr)
      }

      const bundleCacheKey = makeBundleCacheKey(transport.chain, gameAccount.bundleAddr)

      const gameBundle = await getGameBundle(transport, storage, bundleCacheKey, gameAccount.bundleAddr)

      const transactorAddr = gameAccount.transactorAddr
      if (transactorAddr === undefined) {
        throw SdkError.gameNotServed(gameAddr)
      }
      console.log('Transactor address:', transactorAddr)
      const transactorAccount = await transport.getServerAccount(transactorAddr)
      if (transactorAccount === undefined) {
        throw SdkError.transactorAccountNotFound(transactorAddr)
      }
      const decryptionCache = new DecryptionCache()
      const endpoint = transactorAccount.endpoint
      console.log('Transactor endpoint:', endpoint)
      const connection = Connection.initialize(gameAddr, playerAddr, endpoint, encryptor)
      const client = new Client(playerAddr, encryptor, connection)
      const handler = await Handler.initialize(gameBundle, encryptor, client, decryptionCache)

      const getCheckpointParams = new GetCheckpointParams({
        settleVersion: gameAccount.settleVersion,
      })

      let checkpointOffChain: CheckpointOffChain | undefined
      if (gameAccount.checkpointOnChain !== undefined) {
        checkpointOffChain = await connection.getCheckpoint(getCheckpointParams)
        if (checkpointOffChain === undefined) {
          throw new Error('No checkpoint from transactor.')
        }
      }

      console.log('Get checkpoint onchain from game account:', gameAccount.checkpointOnChain)
      console.log('Get checkpoint offchain from transactor:', checkpointOffChain)
      let checkpoint
      if (checkpointOffChain !== undefined && gameAccount.checkpointOnChain !== undefined) {
        checkpoint = Checkpoint.fromParts(checkpointOffChain, gameAccount.checkpointOnChain)
      } else {
        checkpoint = Checkpoint.default()
      }

      const gameContext = new GameContext(gameAccount, checkpoint)
      console.log('Game Context:', clone(gameContext))
      const token = await transport.getToken(gameAccount.tokenAddr)
      if (token === undefined) {
        throw SdkError.tokenNotFound(gameAccount.tokenAddr)
      }
      const info = makeGameInfo(gameAccount, token)
      const profileLoader = new ProfileLoader(transport, storage, onProfile)
      profileLoader.start()

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
        encryptor,
        info,
        decryptionCache,
        profileLoader,
        storage,
        endpoint,
      })
    } finally {
      console.groupEnd()
    }
  }

  async subClient(opts: SubClientInitOpts): Promise<SubClient> {
    try {
      const { gameId, onEvent, onMessage, onTxState, onConnectionState, onError } = opts

      const addr = `${this.__gameAddr}:${gameId.toString()}`

      console.group(`SubClient initialization, id: ${gameId}`)
      console.log('Parent Game Context:', clone(this.__gameContext))

      // Query the on-chain game account to get the latest checkpoint.
      const gameAccount = await this.__getGameAccount()
      const checkpointOnChain = gameAccount.checkpointOnChain
      const settleVersion = gameAccount.settleVersion

      const subGame = this.__gameContext.findSubGame(gameId)

      if (subGame === undefined) {
        console.warn('Game context:', this.__gameContext)
        throw SdkError.invalidSubId(gameId)
      } else {
        console.log('Sub Game:', subGame)
      }

      const bundleAddr = subGame.bundleAddr

      const bundleCacheKey = makeBundleCacheKey(this.__transport.chain, bundleAddr)

      const decryptionCache = new DecryptionCache()
      const playerAddr = this.__wallet.walletAddr
      const connection = Connection.initialize(addr, playerAddr, this.__endpoint, this.__encryptor)
      const client = new Client(playerAddr, this.__encryptor, connection)
      const gameBundle = await getGameBundle(this.__transport, this.__storage, bundleCacheKey, bundleAddr)
      const handler = await Handler.initialize(gameBundle, this.__encryptor, client, decryptionCache)

      // Reload the latest checkpoint.
      const checkpointOffChain = await this.__connection.getCheckpoint(new GetCheckpointParams({ settleVersion }))
      if (checkpointOffChain !== undefined && checkpointOnChain !== undefined) {
        this.__gameContext.checkpoint = Checkpoint.fromParts(checkpointOffChain, checkpointOnChain)
      } else {
        this.__gameContext.checkpoint = Checkpoint.default()
      }

      const gameContext = this.__gameContext.subContext(subGame)
      console.log("SubGame's GameContext:", clone(gameContext))

      return new SubClient({
        gameAddr: addr,
        wallet: this.__wallet,
        transport: this.__transport,
        encryptor: this.__encryptor,
        latestCheckpointOnChain: checkpointOnChain,
        onEvent,
        onMessage,
        onTxState,
        onConnectionState,
        onError,
        handler,
        connection,
        client,
        info: this.__info,
        decryptionCache,
        gameContext,
        gameId,
      })
    } finally {
      console.groupEnd()
    }
  }

  /**
   * Connect to the transactor and retrieve the event stream.
   */
  async attachGame() {
    console.group('Attach to game')
    let sub
    try {
      await this.__attachGameWithRetry()
      sub = this.__connection.subscribeEvents()
      await this.__startSubscribe()
      for (const p of this.__latestGameAccount.players) {
        this.__onLoadProfile(p.accessVersion, p.addr)
      }
    } catch (e) {
      console.error(this.__logPrefix + 'Attaching game failed', e)
      this.__invokeErrorCallback('attach-failed')
      throw e
    } finally {
      console.groupEnd()
    }
    if (sub !== undefined) await this.__processSubscription(sub)
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
    console.log('Use game bundle from cache:', gameBundle)
    if (gameBundle !== undefined) {
      Object.assign(gameBundle, { data: Uint8Array.of() })
    }
  }
  if (gameBundle === undefined) {
    gameBundle = await transport.getGameBundle(bundleAddr)
    console.log('Game bundle:', gameBundle)
    if (gameBundle !== undefined && storage !== undefined && gameBundle.data.length === 0) {
      setTtlCache(storage, bundleCacheKey, gameBundle, BUNDLE_CACHE_TTL)
    }
  }
  if (gameBundle === undefined) {
    throw SdkError.gameBundleNotFound(bundleAddr)
  }
  return gameBundle
}

export function makeGameInfo(gameAccount: GameAccount, token: IToken): GameInfo {
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

  if (gameAccount.entryType instanceof EntryTypeCash) {
    info.minDeposit = gameAccount.entryType.minDeposit
    info.maxDeposit = gameAccount.entryType.maxDeposit
  }

  return info
}
