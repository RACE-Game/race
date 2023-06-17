import { GameAccount, GameRegistration, IToken, PlayerProfile, TokenWithBalance } from './accounts';
import { CreateGameAccountParams, ITransport } from './transport';
import { IWallet } from './wallet';

/**
 * The helper for common interaction.
 *
 * @public
 * @beta
 */
export class AppHelper {
  #transport: ITransport;

  constructor(transport: ITransport) {
    this.#transport = transport;
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
    if (params.minDeposit <= 0) {
      throw new Error('Invalid minDeposit');
    }
    if (params.maxDeposit < params.minDeposit) {
      throw new Error('Invalid maxDeposit');
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
  async listTokens(): Promise<Token[]> {
    return await this.#transport.listTokens();
  }

  /**
   * Fetch balances for a list of tokens.
   *
   * @param walletAddr - The player's wallet address
   * @param tokens - A list of tokens to query
   *
   * @return The list of tokens with `amount` and `uiAmount` added.
   */
  async fetchTokenBalances(walletAddr: string, tokens: IToken[]): Promise<TokenWithBalance[]> {
    const tokenAddrs = tokens.map(t => t.addr);
    const balanceMap = await this.#transport.fetchBalances(walletAddr, tokenAddrs);
    return tokens.map(t => {
      let balance = balanceMap.get(t.addr);
      if (balance === undefined) {
        balance = 0n;
      }
      return new TokenWithBalance(t, balance);
    })
  }
}
