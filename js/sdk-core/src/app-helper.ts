import { EntryTypeCash, GameAccount, GameRegistration, INft, IToken, PlayerProfile, TokenWithBalance } from './accounts';
import { IStorage, getTtlCache, setTtlCache } from './storage';
import { CreateGameAccountParams, ITransport } from './transport';
import { IWallet } from './wallet';


export type AppHelperInitOpts = {
  transport: ITransport,
  storage?: IStorage,
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
  async createGame(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
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

    let addr = await this.#transport.createGameAccount(wallet, params);
    console.debug('Game account created at %s', addr);
    return addr;
  }

  /**
   * Register a game to a registration account.
   *
   * @param wallet - The wallet adapter to sign the transaction
   * @param gameAddr - The address of game account.
   * @param regAddr - The address of registration account.
   */
  async registerGame(wallet: IWallet, gameAddr: string, regAddr: string) {
    await this.#transport.registerGame(wallet, {
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
  async createProfile(wallet: IWallet, nick: string, pfp: string | undefined) {
    await this.#transport.createPlayerProfile(wallet, {
      nick,
      pfp,
    });
  }

  /**
   * Get a player profile.
   *
   * @param addr - The address of player profile account
   * @returns The player profile account or undefined when not found
   */
  async getProfile(addr: string): Promise<PlayerProfile | undefined> {
    return await this.#transport.getPlayerProfile(addr);
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
   * List available tokens.
   *
   * @return A list of token info.
   */
  async listTokens(): Promise<IToken[]> {
    return await this.#transport.listTokens(this.#storage);
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
   * Fetch tokens and balances
   *
   * @param walletAddr - The player's wallet address
   *
   * @return The list of tokens with `amount` and `uiAmount` added.
   */
  async listTokensWithBalance(walletAddr: string): Promise<TokenWithBalance[]> {
    const tokens = await this.listTokens();
    const tokenAddrs = tokens.map(t => t.addr);
    const balanceMap = await this.#transport.fetchBalances(walletAddr, tokenAddrs);
    return tokens.map(t => {
      let balance = balanceMap.get(t.addr);
      if (balance === undefined) {
        balance = 0n;
      }
      return new TokenWithBalance(t, balance);
    });
  }
}
