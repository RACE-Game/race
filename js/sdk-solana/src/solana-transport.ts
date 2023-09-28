import { SystemProgram, Connection, PublicKey, Transaction, Keypair } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
  AccountLayout,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
  createInitializeAccountInstruction,
  createTransferInstruction,
  getAssociatedTokenAddressSync,
  getMint,
} from '@solana/spl-token';
import {
  IWallet,
  ITransport,
  CreateGameAccountParams,
  CloseGameAccountParams,
  JoinParams,
  DepositParams,
  VoteParams,
  CreatePlayerProfileParams,
  PublishGameParams,
  CreateRegistrationParams,
  RegisterGameParams,
  UnregisterGameParams,
  GameAccount,
  GameBundle,
  PlayerProfile,
  ServerAccount,
  RegistrationAccount,
  IToken,
  INft,
  RegistrationWithGames,
  RecipientAccount,
  RecipientClaimParams,
  EntryTypeCash,
  IStorage,
  getTtlCache,
  setTtlCache,
} from '@race-foundation/sdk-core';
import * as instruction from './instruction';

import { GAME_ACCOUNT_LEN, NAME_LEN, PROFILE_ACCOUNT_LEN, PLAYER_PROFILE_SEED, SERVER_PROFILE_SEED } from './constants';

import { GameState, PlayerState, RecipientState, RegistryState, ServerState } from './accounts';

import { join } from './instruction';
import { PROGRAM_ID, METAPLEX_PROGRAM_ID } from './constants';
import { Metadata } from './metadata';

const TOKEN_CACHE_TTL = 24 * 3600;
const NFT_CACHE_TTL = 24 * 30 * 3600;

function trimString(s: string): string {
  return s.replace(/\0/g, '');
}

type LegacyToken = {
  name: string;
  symbol: string;
  logoURI: string;
  address: string;
  decimals: number;
};

export class SolanaTransport implements ITransport {
  #conn: Connection;
  #legacyTokens?: LegacyToken[];

  constructor(endpoint: string) {
    this.#conn = new Connection(endpoint, 'confirmed');
  }

  get chain() {
    return 'Solana';
  }

  async _fetchLegacyTokens() {
    const resp = await fetch('https://cdn.jsdelivr.net/gh/solflare-wallet/token-list/solana-tokenlist.json');
    const m = await resp.json();
    this.#legacyTokens = m['tokens'];
  }

  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
    const { title, bundleAddr, tokenAddr } = params;
    if (title.length > NAME_LEN) {
      // FIXME: better error message?
      throw Error('Game title length exceeds 16 chars');
    }

    const conn = this.#conn;
    const payerKey = new PublicKey(wallet.walletAddr);
    console.log('Payer publick key: ', payerKey);

    let tx = await makeTransaction(conn, payerKey);

    const gameAccount = Keypair.generate();
    const gameAccountKey = gameAccount.publicKey;
    const lamports = await conn.getMinimumBalanceForRentExemption(GAME_ACCOUNT_LEN);
    const createGameAccount = SystemProgram.createAccount({
      fromPubkey: payerKey,
      newAccountPubkey: gameAccountKey,
      lamports: lamports,
      space: GAME_ACCOUNT_LEN,
      programId: PROGRAM_ID,
    });
    tx.add(createGameAccount);

    const tokenMintKey = new PublicKey(tokenAddr);
    const stakeAccount = Keypair.generate();
    const stakeAccountKey = stakeAccount.publicKey;
    const stakeLamports = await conn.getMinimumBalanceForRentExemption(AccountLayout.span);
    const createStakeAccount = SystemProgram.createAccount({
      fromPubkey: payerKey,
      newAccountPubkey: stakeAccountKey,
      lamports: stakeLamports,
      space: AccountLayout.span,
      programId: TOKEN_PROGRAM_ID,
    });
    tx.add(createStakeAccount);

    const initStakeAccount = createInitializeAccountInstruction(
      stakeAccountKey,
      tokenMintKey,
      payerKey,
      TOKEN_PROGRAM_ID
    );
    tx.add(initStakeAccount);

    const bundleKey = new PublicKey(bundleAddr);
    const createGame = instruction.createGameAccount({
      ownerKey: payerKey,
      gameAccountKey: gameAccountKey,
      stakeAccountKey: stakeAccountKey,
      mint: tokenMintKey,
      gameBundleKey: bundleKey,
      title: title,
      maxPlayers: params.maxPlayers,
      entryType: params.entryType,
    });
    tx.add(createGame);
    tx.partialSign(gameAccount, stakeAccount);
    await wallet.sendTransaction(tx, conn);

    return gameAccountKey.toBase58();
  }

  async closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void> {}

  async join(wallet: IWallet, params: JoinParams): Promise<void> {
    const conn = this.#conn;
    const { gameAddr, amount: amountRaw, accessVersion: accessVersionRaw, position, verifyKey } = params;

    const accessVersion = BigInt(accessVersionRaw);
    const playerKey = new PublicKey(wallet.walletAddr);
    const gameAccountKey = new PublicKey(gameAddr);
    const gameState = await this._getGameState(gameAccountKey);
    if (gameState === undefined) {
      throw new Error('Game account not found');
    }

    if (!(gameState.entryType instanceof EntryTypeCash)) {
      throw new Error('Unsupported entry type');
    }
    const mintKey = gameState.tokenKey;
    const isWsol = mintKey.equals(NATIVE_MINT);
    const amount = BigInt(amountRaw);

    if (amount < gameState.entryType.minDeposit || amount > gameState.entryType.maxDeposit) {
      console.log(
        'Max deposit = {}, min deposit = {}, join amount = {}',
        gameState.entryType.maxDeposit,
        gameState.entryType.minDeposit,
        amount
      );
      throw new Error('Join with invalid amount');
    }

    const stakeAccountKey = gameState.stakeKey;
    const tempAccountKeypair = Keypair.generate();
    const tempAccountKey = tempAccountKeypair.publicKey;
    const tempAccountLen = AccountLayout.span;
    const tempAccountLamports = await conn.getMinimumBalanceForRentExemption(tempAccountLen);

    const tx = await makeTransaction(conn, playerKey);
    const createTempAccountIx = SystemProgram.createAccount({
      fromPubkey: playerKey,
      newAccountPubkey: tempAccountKey,
      lamports: tempAccountLamports,
      space: tempAccountLen,
      programId: TOKEN_PROGRAM_ID,
    });
    tx.add(createTempAccountIx);

    const initTempAccountIx = createInitializeAccountInstruction(tempAccountKey, mintKey, playerKey);
    tx.add(initTempAccountIx);

    if (isWsol) {
      const transferAmount = amount - BigInt(tempAccountLamports);
      const transferIx = SystemProgram.transfer({
        fromPubkey: playerKey,
        toPubkey: tempAccountKey,
        lamports: transferAmount,
      });
      tx.add(transferIx);
    } else {
      const playerAta = getAssociatedTokenAddressSync(mintKey, playerKey);
      const transferIx = createTransferInstruction(playerAta, tempAccountKey, playerKey, amount);
      tx.add(transferIx);
    }

    const joinGameIx = await join({
      playerKey,
      paymentKey: tempAccountKey,
      gameAccountKey,
      mint: mintKey,
      stakeAccountKey: stakeAccountKey,
      amount,
      accessVersion,
      position,
      verifyKey,
    });
    tx.add(joinGameIx);

    tx.partialSign(tempAccountKeypair);

    await wallet.sendTransaction(tx, this.#conn);
  }

  async deposit(wallet: IWallet, params: DepositParams): Promise<void> {}

  async publishGame(wallet: IWallet, params: PublishGameParams): Promise<string> {
    return '';
  }

  async vote(wallet: IWallet, params: VoteParams): Promise<void> {}

  async recipientClaim(wallet: IWallet, params: RecipientClaimParams): Promise<void> {

  }

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<void> {
    const { nick, pfp } = params;
    if (nick.length > 16) {
      // FIXME: better error message?
      throw new Error('Player nick name exceeds 16 chars');
    }

    const conn = this.#conn;
    const payerKey = new PublicKey(wallet.walletAddr);
    console.log('Payer Public Key:', payerKey);

    const profileKey = await PublicKey.createWithSeed(payerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);
    console.log('Player profile public key: ', profileKey);

    let tx = new Transaction();

    if (!(await conn.getAccountInfo(profileKey))) {
      let lamports = await conn.getMinimumBalanceForRentExemption(PROFILE_ACCOUNT_LEN);

      const createProfileAccount = SystemProgram.createAccountWithSeed({
        fromPubkey: payerKey,
        newAccountPubkey: profileKey,
        basePubkey: payerKey,
        seed: PLAYER_PROFILE_SEED,
        lamports: lamports,
        space: PROFILE_ACCOUNT_LEN,
        programId: PROGRAM_ID,
      });
      tx.add(createProfileAccount);
    }

    const pfpKey = !pfp ? PublicKey.default : new PublicKey(pfp);
    const createProfile = instruction.createPlayerProfile(payerKey, profileKey, nick, pfpKey);

    tx.add(createProfile);
    await wallet.sendTransaction(tx, conn);
  }

  async createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<string> {
    return '';
  }

  async registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void> {}

  async unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void> {}

  async getGameAccount(addr: string): Promise<GameAccount | undefined> {
    const gameAccountKey = new PublicKey(addr);
    const gameState = await this._getGameState(gameAccountKey);
    if (gameState !== undefined) {
      return gameState.generalize(new PublicKey(addr));
    } else {
      return undefined;
    }
  }

  async getGameBundle(addr: string): Promise<GameBundle | undefined> {
    const mintKey = new PublicKey(addr);
    const [metadataKey] = PublicKey.findProgramAddressSync(
      [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
      METAPLEX_PROGRAM_ID
    );
    const metadataAccount = await this.#conn.getAccountInfo(metadataKey);

    if (metadataAccount === null) {
      return undefined;
    }
    const metadataState = Metadata.deserialize(metadataAccount.data);
    console.log('Metadata state:', metadataState);
    let { uri, name } = metadataState.data;
    // URI should contains the wasm property
    let resp = await fetch(trimString(uri));
    let json = await resp.json();

    let files: any[] = json['properties']['files'];
    let wasm_file = files.find(f => f['type'] == 'application/wasm');

    return new GameBundle({ uri: wasm_file['uri'], name: trimString(name), data: new Uint8Array(0) });
  }

  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    const conn = this.#conn;
    const playerKey = new PublicKey(addr);
    const profileKey = await PublicKey.createWithSeed(playerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);

    const profileAccount = await conn.getAccountInfo(profileKey);

    if (profileAccount) {
      const profileAccountData = profileAccount.data;
      const state = PlayerState.deserialize(profileAccountData);
      return state.generalize(playerKey);
    } else {
      return undefined;
    }
  }

  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    const serverKey = new PublicKey(addr);
    const serverState = await this._getServerState(serverKey);
    if (serverState !== undefined) {
      return serverState.generalize();
    } else {
      return undefined;
    }
  }

  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    const regKey = new PublicKey(addr);
    const regState = await this._getRegState(regKey);
    if (regState !== undefined) {
      return regState.generalize(regKey);
    } else {
      return undefined;
    }
  }

  async getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined> {
    const regAccount = await this.getRegistration(addr);
    if (regAccount === undefined) return undefined;
    const keys = regAccount.games.map(g => new PublicKey(g.addr));
    const gameStates = await this._getMultiGameStates(keys);
    let games: Array<GameAccount | undefined> = [];
    for (let i = 0; i < gameStates.length; i++) {
      const gs = gameStates[i];
      if (gs === undefined) {
        games.push(undefined);
      } else {
        games.push(gs.generalize(keys[i]));
      }
    }
    return new RegistrationWithGames({
      ...regAccount,
      games,
    });
  }

  async getRecipient(addr: String): Promise<RecipientAccount | undefined> {
    const recipientKey = new PublicKey(addr);
    const recipientState = await this._getRecipientState(recipientKey);
    if (recipientState !== undefined) {
      return recipientState.generalize();
    } else {
      return undefined;
    }
  }

  async _fetchImageFromDataUri(dataUri: string): Promise<string | undefined> {
    try {
      const resp = await fetch(dataUri);
      const data = await resp.json();
      return data.image;
    } catch (e) {
      return undefined;
    }
  }

  async getToken(addr: string): Promise<IToken | undefined> {
    const mintKey = new PublicKey(addr);
    try {
      const mint = await getMint(this.#conn, mintKey, 'finalized');
      const [metadataKey] = PublicKey.findProgramAddressSync(
        [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
        METAPLEX_PROGRAM_ID
      );
      const metadataAccount = await this.#conn.getAccountInfo(metadataKey);
      let metadataState;
      if (metadataAccount !== null) {
        metadataState = Metadata.deserialize(metadataAccount.data);
      }

      // Get from legacy token
      if (this.#legacyTokens === undefined) {
        await this._fetchLegacyTokens();
      }
      let legacyToken: LegacyToken | undefined = undefined;
      if (this.#legacyTokens !== undefined) {
        legacyToken = this.#legacyTokens.find(t => t.address === addr);
      }

      if (metadataState !== undefined) {
        const addr = mint.address.toBase58();
        const decimals = mint.decimals;
        const image = await this._fetchImageFromDataUri(metadataState.data.uri);
        const name = metadataState.data.name
          ? trimString(metadataState.data.name)
          : legacyToken
            ? legacyToken.name
            : '';
        const symbol = metadataState.data.symbol
          ? trimString(metadataState.data.symbol)
          : legacyToken
            ? legacyToken.symbol
            : '';
        const icon = image ? image : legacyToken?.logoURI ? legacyToken.logoURI : '';
        return { addr, decimals, name, symbol, icon };
      } else {
        return undefined;
      }
    } catch (e) {
      console.warn(e)
      return undefined;
    }
  }

  /**
   * List popular tokens.
   *
   * [USDT, USDC, SOL, RACE]
   */
  async listTokens(storage?: IStorage): Promise<IToken[]> {
    const popularTokenAddrs = [
      'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      'So11111111111111111111111111111111111111112',
    ];

    let tokens = [];
    for (const addr of popularTokenAddrs) {
      const cacheKey = `TOKEN_CACHE__${this.chain}__${addr}`;

      // Read token info from cache
      if (storage !== undefined) {
        const tokenInfo: IToken | undefined = getTtlCache(storage, cacheKey)
        if (tokenInfo !== undefined) {
          tokens.push(tokenInfo);
          continue;
        }
      }

      // Read on-chain data
      const tokenInfo = await this.getToken(addr);
      if (tokenInfo !== undefined) {
        tokens.push(tokenInfo);

        // Save to cache
        if (storage !== undefined) {
          setTtlCache(storage, cacheKey, tokenInfo, TOKEN_CACHE_TTL);
        }
      }
    }
    return tokens;
  }

  async fetchBalances(walletAddr: string, tokenAddrs: string[]): Promise<Map<string, bigint>> {
    const walletKey = new PublicKey(walletAddr);
    let ret = new Map<string, bigint>();
    for (const tokenAddr of tokenAddrs) {
      const tokenAccountKey = getAssociatedTokenAddressSync(new PublicKey(tokenAddr), walletKey);
      try {
        const resp = await this.#conn.getTokenAccountBalance(tokenAccountKey);
        ret.set(tokenAddr, BigInt(resp.value.amount));
      } catch (e) {
        ret.set(tokenAddr, 0n);
      }
    }
    return ret;
  }

  async getNft(addr: string | PublicKey, storage?: IStorage): Promise<INft | undefined> {
    let mintKey: PublicKey;
    let cacheKey: string;

    if (addr instanceof PublicKey) {
      mintKey = addr;
      cacheKey = `NFT_CACHE__${this.chain}__${addr.toBase58()}`;
    } else {
      mintKey = new PublicKey(addr);
      cacheKey = `NFT_CACHE__${this.chain}__${addr}`;
    }

    // Get nft data from cache
    if (storage !== undefined) {
      const nft: INft | undefined = getTtlCache(storage, cacheKey);
      if (nft !== undefined) {
        return nft;
      }
    }

    try {
      const mint = await getMint(this.#conn, mintKey, 'finalized');

      // Non-zero decimals stands for a fungible token
      if (mint.decimals !== 0) {
        return undefined;
      }

      const [metadataKey] = PublicKey.findProgramAddressSync(
        [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
        METAPLEX_PROGRAM_ID
      );
      const metadataAccount = await this.#conn.getAccountInfo(metadataKey);
      let metadataState;
      if (metadataAccount !== null) {
        metadataState = Metadata.deserialize(metadataAccount.data);
      }
      if (metadataState !== undefined) {
        const image = await this._fetchImageFromDataUri(metadataState.data.uri);
        if (image === undefined) return undefined;

        const nft = {
          addr: mint.address.toBase58(),
          name: trimString(metadataState.data.name),
          symbol: trimString(metadataState.data.symbol),
          image,
          collection: metadataState?.collection?.key.toBase58(),
        };

        if (storage !== undefined) {
          setTtlCache(storage, cacheKey, nft, NFT_CACHE_TTL);
        }
        return nft;
      } else {
        return undefined;
      }
    } catch (e) {
      console.warn(e);
      return undefined;
    }
  }

  async listNfts(walletAddr: string, storage?: IStorage): Promise<INft[]> {
    let nfts = [];
    const ownerKey = new PublicKey(walletAddr);
    const parsedTokenAccounts = await this.#conn.getParsedTokenAccountsByOwner(ownerKey, {
      programId: TOKEN_PROGRAM_ID,
    });
    for (const a of parsedTokenAccounts.value) {
      if (
        a.account.data.parsed.info.tokenAmount.amount !== '1' ||
          a.account.data.parsed.info.tokenAmount.decimals !== 0
      ) {
        continue;
      }

      const nft = await this.getNft(a.account.data.parsed.info.mint, storage);
      if (nft !== undefined) {
        nfts.push(nft);
      }
    }
    return nfts;
  }

  async _getGameState(gameAccountKey: PublicKey): Promise<GameState | undefined> {
    const conn = this.#conn;
    const gameAccount = await conn.getAccountInfo(gameAccountKey);
    if (gameAccount !== null) {
      const data = gameAccount.data;
      return GameState.deserialize(data);
    } else {
      return undefined;
    }
  }

  async _getRecipientState(recipientAccountKey: PublicKey): Promise<RecipientState | undefined> {
    const conn = this.#conn;
    const recipientAccount = await conn.getAccountInfo(recipientAccountKey);
    if (recipientAccount !== null) {
      const data = recipientAccount.data;
      return RecipientState.deserialize(data);
    } else {
      return undefined;
    }
  }

  async _getMultiGameStates(gameAccountKeys: PublicKey[]): Promise<Array<GameState | undefined>> {
    const conn = this.#conn;
    const accountsInfo = await conn.getMultipleAccountsInfo(gameAccountKeys);
    const ret: Array<GameState | undefined> = [];
    console.info("Get %s games from registry", accountsInfo.length)
    for (let i = 0; i < accountsInfo.length; i++) {
      const key = gameAccountKeys[i];
      const accountInfo = accountsInfo[i];
      if (accountInfo !== null) {
        try {
          ret.push(GameState.deserialize(accountInfo.data));
          console.info("Found game account %s", key);
        } catch(_: any) {
          ret.push(undefined);
          console.warn("Skip invalid game account %s", key);
        }
      } else {
        ret.push(undefined);
        console.warn("Game account %s not exist", key);
      }
    }
    return ret;
  }

  async _getRegState(regKey: PublicKey): Promise<RegistryState | undefined> {
    const conn = this.#conn;
    const regAccount = await conn.getAccountInfo(regKey);
    if (regAccount !== null) {
      const data = regAccount.data;
      return RegistryState.deserialize(data);
    } else {
      return undefined;
    }
  }

  async _getServerState(serverKey: PublicKey): Promise<ServerState | undefined> {
    const conn = this.#conn;
    const profileKey = await PublicKey.createWithSeed(serverKey, SERVER_PROFILE_SEED, PROGRAM_ID);
    const serverAccount = await conn.getAccountInfo(profileKey);
    if (serverAccount !== null) {
      const data = serverAccount.data;
      return ServerState.deserialize(data);
    } else {
      return undefined;
    }
  }
}

async function makeTransaction(conn: Connection, feePayer: PublicKey): Promise<Transaction> {
  const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash();
  return new Transaction({
    feePayer,
    blockhash,
    lastValidBlockHeight,
  });
}
