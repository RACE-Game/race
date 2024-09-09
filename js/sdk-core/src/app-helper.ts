import { EntryTypeCash, GameAccount, INft, IToken, ITokenWithBalance, PlayerProfile, RecipientAccount, TokenWithBalance } from './accounts';
import { IStorage } from './storage';
import { CreateGameAccountParams, ITransport, TransactionResult } from './transport';
import { PlayerProfileWithPfp } from './types';
import { IWallet } from './wallet';


export type AppHelperInitOpts = {
  transport: ITransport,
  storage?: IStorage,
};

export type ClaimPreview = {
  tokenAddr: string,
  amount: bigint,
};

/**
 * The helper for common interaction.
 *
 * @public
 * @beta
 */
export class AppHelper {
  #transport: ITransport;
  #storage?: IStorage;

  constructor(transport: ITransport)
  constructor(opts: AppHelperInitOpts)
  constructor(transportOrOpts: ITransport | AppHelperInitOpts) {
    if ('transport' in transportOrOpts) {
      const { transport, storage } = transportOrOpts;
      this.#transport = transport;
      this.#storage = storage;
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
    return await this.#transport.getGameAccount(addr);
  }

  /**
   * Create a game account.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param params - Parameters for game creation
   * @returns The address of created game
   */
  async createGame(wallet: IWallet, params: CreateGameAccountParams): Promise<TransactionResult<string>> {
    if (params.title.length == 0 || params.title.length > 16) {
      throw new Error('Invalid title');
    }

    if (params.entryType instanceof EntryTypeCash) {
      const entryType = params.entryType;
      if (entryType.minDeposit <= 0) {
        throw new Error('Invalid minDeposit');
      }
      if (entryType.maxDeposit < entryType.minDeposit) {
        throw new Error('Invalid maxDeposit');
      }
    } else {
      throw new Error('Unsupported entry type');
    }

    if (params.maxPlayers < 1 || params.maxPlayers > 512) {
      throw new Error('Invalid maxPlayers');
    }

    let res = await this.#transport.createGameAccount(wallet, params);
    if (res.result === 'ok') {
      console.debug('Game account created at %s', res.value);
    } else {
      console.error('Failed to create game account');
    }
    return res;
  }

  /**
   * Register a game to a registration account.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param gameAddr - The address of game account.
   * @param regAddr - The address of registration account.
   */
  async registerGame(wallet: IWallet, gameAddr: string, regAddr: string): Promise<TransactionResult<void>> {
    return await this.#transport.registerGame(wallet, {
      gameAddr,
      regAddr,
    });
  }

  /**
   * Create a player profile.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param nick - The nick name
   * @param pfp - The address of avatar NFT
   */
  async createProfile(wallet: IWallet, nick: string, pfp: string | undefined): Promise<TransactionResult<void>> {
    const res = await this.#transport.createPlayerProfile(wallet, {
      nick,
      pfp,
    });
    if (res.result === 'ok') {
      console.debug('Created player profile');
    } else {
      console.error('Failed to create player profile');
    }
    return res;
  }

  /**
   * Get a player profile.
   *
   * @param addr - The address of player profile account
   * @returns The player profile account or undefined when not found
   */
  async getProfile(addr: string): Promise<PlayerProfileWithPfp | undefined> {
    const profile = await this.#transport.getPlayerProfile(addr);
    if (profile === undefined) return undefined;
    if (profile.pfp !== undefined) {
      const pfp = await this.#transport.getNft(profile.pfp, this.#storage);
      return { nick: profile.nick, addr: profile.addr, pfp };
    } else {
      return { nick: profile.nick, addr: profile.addr, pfp: undefined };
    }
  }

  /**
   * List games from a list of registration accounts.
   *
   * @param registrationAddrs - The addresses of registration accounts
   * @return A list of games
   */
  async listGames(registrationAddrs: string[]): Promise<GameAccount[]> {
    let games: GameAccount[] = [];
    for (const addr of registrationAddrs) {
      const reg = await this.#transport.getRegistrationWithGames(addr);
      if (reg !== undefined) {
        for (const game of reg.games) {
          games.push(game);
        }
      }
    }
    return games;
  }

  /**
   * List tokens.
   *
   * @return A list of token info.
   */
  async listTokens(tokenAddrs: string[]): Promise<IToken[]> {
    return await this.#transport.listTokens(tokenAddrs, this.#storage);
  }

  /**
   * List tokens with their balance.
   *
   * @return A list of token info.
   */
  async listTokensWithBalance(walletAddr: string, tokenAddrs: string[]): Promise<ITokenWithBalance[]> {
    return await this.#transport.listTokensWithBalance(walletAddr, tokenAddrs, this.#storage);
  }

  /**
   * List all nfts owned by a wallet.
   *
   * @param walletAddr - wallet address.
   * @param collectionName - The collection name for filtering, pass `undefined` for no filtering.
   *
   * @return A list of nfts.
   */
  async listNfts(walletAddr: string, collection: string | undefined = undefined): Promise<INft[]> {
    const nfts = await this.#transport.listNfts(walletAddr, this.#storage);
    if (collection === undefined) {
      return nfts;
    } else {
      return nfts.filter(nft => nft.collection === collection);
    }
  }

  /**
   * Get NFT by address
   *
   * @param addr - The address of NFT
   */
  async getNft(addr: string): Promise<INft | undefined> {
    return await this.#transport.getNft(addr, this.#storage)
  }

  /**
   * Claim the fees collected by game.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param gameAddr - The address of game account.
   */
  async claim(wallet: IWallet, gameAddr: string): Promise<TransactionResult<void>> {
    const gameAccount = await this.#transport.getGameAccount(gameAddr);
    if (gameAccount === undefined) throw new Error('Game account not found');
    return await this.#transport.recipientClaim(wallet, { recipientAddr: gameAccount?.recipientAddr });
  }

  async getRecipient(recipientAddr: string): Promise<RecipientAccount | undefined> {
    return await this.#transport.getRecipient(recipientAddr);
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
      const r = await this.#transport.getRecipient(recipient);
      if (r === undefined) {
        throw new Error('Recipient account not found');
      }
      recipient = r;
    }

    let ret: ClaimPreview[] = [];
    for (const slot of recipient.slots) {
      let weights = 0;
      let totalWeights = 0;
      let totalClaimed = 0n;
      let claimed = 0n;
      for (const share of slot.shares) {
        totalClaimed += share.claimAmount;
        totalWeights += share.weights;
        if (share.owner === wallet.walletAddr) {
          weights += share.weights;
          claimed += share.claimAmount;
        }
      }
      const totalAmount = totalClaimed + slot.balance;
      const amountToClaim = BigInt(Number(totalAmount) * weights / totalWeights) - claimed;
      if (amountToClaim > 0n) {
        ret.push({
          amount: amountToClaim,
          tokenAddr: slot.tokenAddr
        });
      }
    }
    return ret;
  }
}
