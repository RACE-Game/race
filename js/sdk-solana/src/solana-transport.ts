import {
  SystemProgram,
  Connection,
  PublicKey,
  Transaction,
  clusterApiUrl,
  TransactionInstruction,
  sendAndConfirmTransaction,
  Keypair,
} from '@solana/web3.js';
import {
  AccountLayout,
  NATIVE_MINT,
  NATIVE_MINT_2022,
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
} from '@race-foundation/sdk-core';
import * as instruction from './instruction';

import { GAME_ACCOUNT_LEN, NAME_LEN, PROFILE_ACCOUNT_LEN, PLAYER_PROFILE_SEED, SERVER_PROFILE_SEED } from './constants';

import { GameState, PlayerState, RegistryState, ServerState } from './accounts';

import { SolanaWalletAdapter } from './solana-wallet';
import { join } from './instruction';
import { PROGRAM_ID, METAPLEX_PROGRAM_ID } from './constants';
import { Metadata } from './metadata';

export class SolanaTransport implements ITransport {
  #conn: Connection;

  constructor(endpoint: string) {
    this.#conn = new Connection(endpoint, 'confirmed');
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
      minDeposit: params.minDeposit,
      maxDeposit: params.maxDeposit,
    });
    tx.add(createGame);
    tx.partialSign(gameAccount, stakeAccount);
    await wallet.sendTransaction(tx, conn);

    return gameAccountKey.toBase58();
  }

  async closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void> { }

  async join(wallet: IWallet, params: JoinParams): Promise<void> {
    const conn = this.#conn;
    const { gameAddr, amount: amountRaw, accessVersion: accessVersionRaw, position, verifyKey } = params;

    const accessVersion = BigInt(accessVersionRaw);
    const playerKey = new PublicKey(wallet.walletAddr);
    const gameAccountKey = new PublicKey(gameAddr);
    const gameState = await this._getGameState(gameAccountKey);
    if (gameState === undefined) {
      throw new Error('TS: Game account not found');
    }
    const mintKey = gameState.tokenKey;
    const isWsol = mintKey.equals(NATIVE_MINT);
    const amount = BigInt(amountRaw);

    if (amount < gameState.minDeposit || amount > gameState.maxDeposit) {
      console.log(
        'Max deposit = {}, min deposit = {}, join amount = {}',
        gameState.maxDeposit,
        gameState.minDeposit,
        amount
      );
      throw new Error('TS: Join with invalid amount');
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
      verifyKey
    });
    tx.add(joinGameIx);

    tx.partialSign(tempAccountKeypair);

    await wallet.sendTransaction(tx, this.#conn);
  }

  async deposit(wallet: IWallet, params: DepositParams): Promise<void> { }

  async publishGame(wallet: IWallet, params: PublishGameParams): Promise<string> {
    return '';
  }

  async vote(wallet: IWallet, params: VoteParams): Promise<void> { }

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

  async registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void> { }

  async unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void> { }

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
    console.log('Game Stake Mint:', mintKey);
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
    uri = uri.replace(/\0/g, '');
    name = name.replace(/\0/g, '');

    console.log('uri:', uri);
    console.log('name:', name);

    return new GameBundle({ uri, name, data: new Uint8Array(0) });
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
    for (let i = 0; i < gameStates.length; i ++) {
      const gs = gameStates[i];
      if (gs === undefined) {
        games.push(undefined);
      } else {
        games.push(gs.generalize(keys[i]));
      }
    }
    return new RegistrationWithGames({
      ...regAccount,
      games
    });
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
      if (metadataState !== undefined) {
        return {
          addr: mint.address.toBase58(),
          decimals: mint.decimals,
          name: metadataState.data.name,
          symbol: metadataState.data.symbol,
          icon: metadataState.data.uri,
        }
      } else {
        return undefined;
      }
    } catch (e) {
      console.warn(e);
      return undefined;
    }
  }

  /**
   * List popular tokens.
   *
   * [USDT, USDC, SOL, RACE]
   */
  async listTokens(): Promise<IToken[]> {
    const popularTokenAddrs = [
      "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "So11111111111111111111111111111111111111112",
      "RACE5fnTKB9obGtCusArTQ6hhdNXAtf3HarvJM17rxJ"
    ];

    let tokens = [];
    for (const addr of popularTokenAddrs) {
      const tokenInfo = await this.getToken(addr);
      if (tokenInfo !== undefined) {
        tokens.push(tokenInfo);
      }
    }
    return tokens;
  }

  async getNft(addr: string): Promise<INft | undefined> {
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
      if (metadataState !== undefined) {
        return {
          addr: mint.address.toBase58(),
          name: metadataState.data.name,
          symbol: metadataState.data.symbol,
          image: metadataState.data.uri,
        }
      } else {
        return undefined;
      }
    } catch (e) {
      console.warn(e);
      return undefined;
    }
  }

  async _getGameState(gameAccoutKey: PublicKey): Promise<GameState | undefined> {
    const conn = this.#conn;
    const gameAccount = await conn.getAccountInfo(gameAccoutKey);
    if (gameAccount !== null) {
      const data = gameAccount.data;
      return GameState.deserialize(data);
    } else {
      return undefined;
    }
  }

  async _getMultiGameStates(gameAccountKeys: PublicKey[]): Promise<Array<GameState | undefined>> {
    const conn = this.#conn;
    const accountsInfo = await conn.getMultipleAccountsInfo(gameAccountKeys);
    const ret: Array<GameState | undefined> = [];
    for (const accountInfo of accountsInfo) {
      if (accountInfo === null) {
        ret.push(undefined);
      } else {
        ret.push(GameState.deserialize(accountInfo.data));
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
      console.log('Server account data:', data);
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
