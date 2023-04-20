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
import { AccountLayout, NATIVE_MINT, NATIVE_MINT_2022, TOKEN_PROGRAM_ID, createInitializeAccountInstruction, createTransferInstruction, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { Metadata, PROGRAM_ID as METAPLEX_PROGRAM_ID } from '@metaplex-foundation/mpl-token-metadata';
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
} from 'race-sdk-core';
import * as instruction from './instruction';

import {
  GAME_ACCOUNT_LEN,
  NAME_LEN,
  PROFILE_ACCOUNT_LEN,
  PLAYER_PROFILE_SEED,
  SERVER_PROFILE_SEED,
} from './constants';

import {
  GameState,
  PlayerState,
  RegistryState,
  ServerState,
} from './accounts'

import { SolanaWalletAdapter } from './solana-wallet';
import { join } from './instruction';

const PROGRAM_ID = new PublicKey('8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu');

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
    console.log("Payer publick key: ", payerKey);

    let tx = new Transaction();

    // Create game account
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

    // Create stake account to hold deposits
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

    const initStakeAccount = createInitializeAccountInstruction(stakeAccountKey, tokenMintKey, payerKey, TOKEN_PROGRAM_ID);
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
    const { gameAddr, amount, accessVersion, position } = params;
    const playerKey = new PublicKey(wallet.walletAddr);
    const gameAccountKey = new PublicKey(gameAddr);
    const gameState = await this._getGameState(gameAccountKey);
    if (gameState === undefined) {
      throw new Error('Game account not found');
    }
    if (gameState.accessVersion !== accessVersion) {
      throw new Error('Access version not match');
    }
    const mintKey = gameState.tokenKey;
    const isWsol = mintKey.equals(NATIVE_MINT);

    const stakeAccountKey = gameState.stakeKey;
    const tempAccountKeypair = Keypair.generate();
    const tempAccountKey = tempAccountKeypair.publicKey;
    const tempAccountLen = AccountLayout.span;
    const tempAccountLamports = await this.#conn.getMinimumBalanceForRentExemption(
      tempAccountLen
    );

    const tx = new Transaction();
    const createTempAccountIx = SystemProgram.createAccount(
      {
        fromPubkey: playerKey,
        newAccountPubkey: tempAccountKey,
        lamports: tempAccountLamports,
        space: tempAccountLen,
        programId: TOKEN_PROGRAM_ID,
      });
    tx.add(createTempAccountIx);

    const initTempAccountIx = createInitializeAccountInstruction(
      tempAccountKey,
      mintKey,
      playerKey
    );
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
      const transferIx = createTransferInstruction(
        playerAta,
        tempAccountKey,
        playerKey,
        amount
      );
      tx.add(transferIx);
    }

    const joinGameIx = join({
      playerKey,
      paymentKey: tempAccountKey,
      gameAccountKey,
      mint: mintKey,
      stakeAccountKey: stakeAccountKey,
      amount,
      accessVersion,
      position,
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

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<string> {
    const { nick, pfp } = params;
    if (nick.length > 16) {
      // FIXME: better error message?
      throw new Error('Player nick name exceeds 16 chars');
    }

    const conn = this.#conn;
    const payerKey = new PublicKey(wallet.walletAddr);
    console.log("Payer Public Key:", payerKey);

    const profileKey = await PublicKey.createWithSeed(payerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);
    console.log('Player profile public key: ', profileKey);

    let tx = new Transaction();

    // Check if player account already exists
    if (!(await conn.getAccountInfo(profileKey))) {
      let lamports = await conn.getMinimumBalanceForRentExemption(PROFILE_ACCOUNT_LEN);

      // Construct ix
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

    // Get pfp ready
    const pfpKey = !pfp ? PublicKey.default : new PublicKey(pfp);

    // Construct ix
    const createProfile = instruction.createPlayerProfile(payerKey, profileKey, nick, pfpKey);

    tx.add(createProfile);

    await wallet.sendTransaction(tx, conn);

    return profileKey.toBase58();
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
    const conn = this.#conn;
    const [metadataKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("metadata", 'utf8'),
      METAPLEX_PROGRAM_ID.toBuffer(),
      mintKey.toBuffer()],
      METAPLEX_PROGRAM_ID);
    const metadataState = await Metadata.fromAccountAddress(conn, metadataKey);
    console.log(metadataState);
    let { uri, name } = metadataState.data;
    uri = uri.replace(/\0/g, '');
    name = name.replace(/\0/g, '');

    console.log("uri:", uri);
    console.log("name:", name);

    const response = await fetch(uri);
    const metadata: any = await response.json();

    console.log("metadata:", metadata);
    const wasm = metadata.properties.files.find((f: any) => f['type'] == "application/wasm");

    const respWasm = await fetch(wasm.uri);
    const data = Array.from(new Uint8Array(await respWasm.arrayBuffer()));

    return { uri, name, data };
  }

  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    const conn = this.#conn;
    const playerKey = new PublicKey(addr);
    const profileKey = await PublicKey.createWithSeed(
      playerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);

    const profileAccount = await conn.getAccountInfo(profileKey);

    if (profileAccount) {
      const profileAccountData = profileAccount.data;

      const { nick, pfpKey } = PlayerState.deserialize(profileAccountData);
      if (pfpKey !== undefined) {
        return { addr: addr, nick: nick, pfp: pfpKey.toBase58() };
      } else {
        return { addr: addr, nick: nick, pfp: undefined }
      }
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
      let x = regState.generalize(regKey);
      console.log(x);
      return x;
    } else {
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
    const profileKey = await PublicKey.createWithSeed(
      serverKey,
      SERVER_PROFILE_SEED,
      PROGRAM_ID,
    );
    const serverAccount = await conn.getAccountInfo(profileKey);
    if (serverAccount !== null) {
      const data = serverAccount.data;
      console.log("Server account data:", data);
      return ServerState.deserialize(data);
    } else {
      return undefined;
    }
  }

}
