import { GameAccount, Nft, Token, TokenWithBalance, RecipientAccount } from './accounts'
import { GAME_ACCOUNT_CACHE_TTL, NFT_CACHE_TTL, TOKEN_CACHE_TTL } from './common'
import { ResponseHandle, ResponseStream } from './response'
import {
  getTtlCache,
  IStorage,
  makeGameAccountCacheKey,
  makeNftCacheKey,
  makeTokenCacheKey,
  setTtlCache,
} from './storage'
import { GameAccountCache, makeGameAccountCache } from './account-cache'
import {
  CreateGameAccountParams,
  CreateGameError,
  CreateGameResponse,
  CreatePlayerProfileError,
  CreatePlayerProfileResponse,
  ITransport,
  RecipientClaimError,
  RecipientClaimResponse,
  RegisterGameError,
  RegisterGameResponse,
} from './transport'
import { PlayerProfileWithPfp } from './types'
import { IWallet } from './wallet'
import { getCheckpoint } from './connection'

export type AppHelperInitOpts = {
  transport: ITransport
  storage?: IStorage
}

export type ClaimPreview = {
  tokenAddr: string
  amount: bigint
}

/**
 * The helper for common interaction.
 */
export class AppHelper {
  #transport: ITransport
  #storage?: IStorage

  constructor(transport: ITransport)
  constructor(opts: AppHelperInitOpts)
  constructor(transportOrOpts: ITransport | AppHelperInitOpts) {
    if ('transport' in transportOrOpts) {
      const { transport, storage } = transportOrOpts
      this.#transport = transport
      this.#storage = storage
    } else {
      this.#transport = transportOrOpts
    }
  }

  /**
   * Get the game account by game address.
   *
   * @param addr - The address of game account
   * @returns An object of GameAccount or undefined when not found
   */
  async getGame(addr: string): Promise<GameAccount | undefined> {
    const game = await this.#transport.getGameAccount(addr)
    if (game === undefined) {
      return undefined
    }
    if (this.#storage !== undefined) {
      const cacheKey = makeGameAccountCacheKey(this.#transport.chain, game.addr)
      setTtlCache(this.#storage, cacheKey, makeGameAccountCache(game), GAME_ACCOUNT_CACHE_TTL)
    }
    return game
  }

  /**
   * Create a game account.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param params - Parameters for game creation
   * @returns The address of created game
   */
  createGame(wallet: IWallet, params: CreateGameAccountParams): ResponseStream<CreateGameResponse, CreateGameError> {
    if (params.title.length == 0 || params.title.length > 16) {
      throw new Error('Invalid title')
    }

    if (params.entryType.kind === 'cash') {
      const entryType = params.entryType
      if (entryType.minDeposit <= 0) {
        throw new Error('Invalid minDeposit')
      }
      if (entryType.maxDeposit < entryType.minDeposit) {
        throw new Error('Invalid maxDeposit')
      }
    } else if (params.entryType.kind === 'ticket') {
      const entryType = params.entryType
      if (entryType.amount <= 0) {
        throw new Error('Invalid ticket price')
      }
    } else {
      throw new Error('Unsupported entry type')
    }

    if (params.maxPlayers < 1 || params.maxPlayers > 512) {
      throw new Error('Invalid maxPlayers')
    }

    let response = new ResponseHandle<CreateGameResponse, CreateGameError>()
    this.#transport.createGameAccount(wallet, params, response)

    return response.stream()
  }

  /**
   * Register a game to a registration account.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param gameAddr - The address of game account.
   * @param regAddr - The address of registration account.
   */
  registerGame(
    wallet: IWallet,
    gameAddr: string,
    regAddr: string
  ): ResponseStream<RegisterGameResponse, RegisterGameError> {
    const response = new ResponseHandle<RegisterGameResponse, RegisterGameError>()
    this.#transport.registerGame(
      wallet,
      {
        gameAddr,
        regAddr,
      },
      response
    )

    return response.stream()
  }

  /**
   * Initiates the creation of a player profile using the provided wallet, nickname, and optional profile picture.
   * @param {IWallet} wallet - The wallet associated with the player.
   * @param {string} nick - The nickname for the player.
   * @param {string | undefined} pfp - The profile picture for the player, if any.
   * @returns {ResponseStream<CreatePlayerProfileResponse, CreatePlayerProfileError>} - A stream of responses indicating the success or failure of the operation.
   */
  createProfile(
    wallet: IWallet,
    nick: string,
    pfp?: string
  ): ResponseStream<CreatePlayerProfileResponse, CreatePlayerProfileError> {
    const response = new ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>()

    this.#transport.createPlayerProfile(wallet, { nick, pfp }, response)

    return response.stream()
  }

  /**
   * Retrieves the checkpoint state for a given address.
   *
   * This function attempts to fetch the checkpoint state of a specific game account
   * based on the provided account address. It checks if storage is set up and if the game
   * cache contains a valid transactor endpoint to retrieve the data from.
   *
   * @param addr - The address of the game account for which the checkpoint state is requested.
   * @returns A promise that resolves to a `Uint8Array` containing the checkpoint state if available, or `undefined`.
   * @throws Will throw an error if the game is not loaded or if storage is not set.
   */
  async getCheckpointState(addr: string): Promise<Uint8Array | undefined> {
    if (this.#storage !== undefined) {
      const cacheKey = makeGameAccountCacheKey(this.#transport.chain, addr)
      const gameCache = getTtlCache<GameAccountCache>(this.#storage, cacheKey)
      if (gameCache !== undefined) {
        if (gameCache.transactorEndpoint === undefined) {
          return undefined // The game is not served
        }
        const checkpointOffChain = await getCheckpoint(gameCache.transactorEndpoint, gameCache.addr, {
          settleVersion: BigInt(gameCache.settleVersion),
        })
        return checkpointOffChain?.data.get(0)?.data
      } else {
        throw new Error('Game not loaded, try listGames or getGame first')
      }
    } else {
      throw new Error('Storage is required')
    }
  }

  /**
   * Get a player profile.
   *
   * @param addr - The address of player profile account
   * @returns The player profile account or undefined when not found
   */
  async getProfile(addr: string): Promise<PlayerProfileWithPfp | undefined> {
    const profile = await this.#transport.getPlayerProfile(addr)
    if (profile === undefined) return undefined
    if (profile.pfp !== undefined) {
      const pfp = await this.getNft(profile.pfp)
      return { nick: profile.nick, addr: profile.addr, pfp }
    } else {
      return { nick: profile.nick, addr: profile.addr, pfp: undefined }
    }
  }

  /**
   * List games from a list of registration accounts.
   *
   * @param registrationAddrs - The addresses of registration accounts
   * @return A list of games
   */
  async listGames(registrationAddrs: string[]): Promise<GameAccount[]> {
    let games: GameAccount[] = []
    for (const addr of registrationAddrs) {
      const reg = await this.#transport.getRegistrationWithGames(addr)
      if (reg !== undefined) {
        for (const game of reg.games) {
          // Save games to cache
          if (this.#storage !== undefined) {
            const cacheKey = makeGameAccountCacheKey(this.#transport.chain, game.addr)
            const gameCache = makeGameAccountCache(game)
            setTtlCache(this.#storage, cacheKey, gameCache, GAME_ACCOUNT_CACHE_TTL)
          }
          games.push(game)
        }
      }
    }
    return games
  }

  /**
   * List tokens.
   *
   * @return A list of token info.
   */
  async listTokens(tokenAddrs: string[]): Promise<Token[]> {
    if (this.#storage === undefined) {
      return await this.#transport.listTokens(tokenAddrs)
    } else {
      let res: Token[] = []
      let queryAddrs: string[] = []
      for (const addr of tokenAddrs) {
        const cacheKey = makeTokenCacheKey(this.#transport.chain, addr)
        const token = getTtlCache<Token>(this.#storage, cacheKey)
        if (token !== undefined) {
          console.debug('Get token info from cache: %s', addr)
          res.push(token)
        } else {
          queryAddrs.push(addr)
        }
      }
      const queryRst = await this.#transport.listTokens(queryAddrs)
      for (const token of queryRst) {
        console.debug('Get token info from transport: %s', token.addr)
        res.push(token)
        setTtlCache(this.#storage, makeTokenCacheKey(this.#transport.chain, token.addr), token, TOKEN_CACHE_TTL)
      }
      return res
    }
  }

  /**
   * List tokens with their balance.
   *
   * @return A list of token info.
   */
  async listTokensWithBalance(walletAddr: string, tokenAddrs: string[]): Promise<TokenWithBalance[]> {
    return await this.#transport.listTokensWithBalance(walletAddr, tokenAddrs)
  }

  /**
   * List all nfts owned by a wallet.
   *
   * @param walletAddr - wallet address.
   * @param collectionName - The collection name for filtering, pass `undefined` for no filtering.
   *
   * @return A list of nfts.
   */
  async listNfts(walletAddr: string, collection: string | undefined = undefined): Promise<Nft[]> {
    const nfts = await this.#transport.listNfts(walletAddr)
    if (collection === undefined) {
      return nfts
    } else {
      return nfts.filter(nft => nft.collection === collection)
    }
  }

  /**
   * Get NFT by address
   *
   * @param addr - The address of NFT
   */
  async getNft(addr: string): Promise<Nft | undefined> {
    if (this.#storage === undefined) {
      return await this.#transport.getNft(addr)
    } else {
      const cacheKey = makeNftCacheKey(this.#transport.chain, addr)
      const cached = getTtlCache<Nft>(this.#storage, cacheKey)
      if (cached !== undefined) {
        return cached
      } else {
        const nft = await this.#transport.getNft(addr)
        if (nft !== undefined) {
          setTtlCache(this.#storage, cacheKey, nft, NFT_CACHE_TTL)
        }
        return nft
      }
    }
  }

  /**
   * Claim the fees collected by game.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param gameAddr - The address of game account.
   */
  claim(wallet: IWallet, recipientAddr: string): ResponseStream<RecipientClaimResponse, RecipientClaimError> {
    const response = new ResponseHandle<RecipientClaimResponse, RecipientClaimError>()
    this.#transport.recipientClaim(wallet, { recipientAddr }, response)
    return response.stream()
  }

  async getRecipient(recipientAddr: string): Promise<RecipientAccount | undefined> {
    return await this.#transport.getRecipient(recipientAddr)
  }

  /**
   * Preview the claim information.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param recipientAddr | recipientAccount - The address of a recipient account.
   */
  previewClaim(wallet: IWallet, recipientAddr: string): Promise<ClaimPreview[]>
  previewClaim(wallet: IWallet, recipientAccount: RecipientAccount): Promise<ClaimPreview[]>
  async previewClaim(wallet: IWallet, recipient: RecipientAccount | string): Promise<ClaimPreview[]> {
    if (typeof recipient === 'string') {
      const r = await this.#transport.getRecipient(recipient)
      if (r === undefined) {
        throw new Error('Recipient account not found')
      }
      recipient = r
    }

    let ret: ClaimPreview[] = []
    for (const slot of recipient.slots) {
      let weights = 0
      let totalWeights = 0
      let totalClaimed = 0n
      let claimed = 0n
      for (const share of slot.shares) {
        totalClaimed += share.claimAmount
        totalWeights += share.weights

        if (share.owner.kind === 'assigned' && share.owner.addr === wallet.walletAddr) {
          weights += share.weights
          claimed += share.claimAmount
        }
      }
      const totalAmount = totalClaimed + slot.balance
      const amountToClaim = BigInt((Number(totalAmount) * weights) / totalWeights) - claimed
      if (amountToClaim > 0n) {
        ret.push({
          amount: amountToClaim,
          tokenAddr: slot.tokenAddr,
        })
      }
    }
    return ret
  }
}
