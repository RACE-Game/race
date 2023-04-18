import {
  SystemProgram,
  Connection,
  PublicKey,
  Transaction,
  clusterApiUrl,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
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
import * as intruction from './instruction';

import {
  PROFILE_ACCOUNT_LEN,
  PLAYER_PROFILE_SEED,
} from './constants';

import {
  PlayerState,
} from './accounts'

const PROGRAM_ID = new PublicKey('8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu');

class SolanaTransport implements ITransport {
  #conn: Connection;

  constructor(endpoint: string) {
    this.#conn = new Connection(endpoint, 'finalized');
  }

  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string> {
    return '';
  }

  async closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void> {}

  async join(wallet: IWallet, params: JoinParams): Promise<void> {}

  async deposit(wallet: IWallet, params: DepositParams): Promise<void> {}

  async publishGame(wallet: IWallet, params: PublishGameParams): Promise<string> {
    return '';
  }

  async vote(wallet: IWallet, params: VoteParams): Promise<void> {}

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<string> {
    const { nick, pfp } = params;
    if (nick.length > 16) {
      // FIXME: better error message?
      throw new Error('Player nick name exceeds 16 chars');
    }

    const payer = wallet.walletAddr;
    const payerKey = new PublicKey(payer);
    const profileKey = await PublicKey.createWithSeed(payerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);
    console.log('Player profile public key: ', profileKey);

    let tx = new Transaction();

    // Check if player account already exists
    if (!(await this.#conn.getAccountInfo(profileKey))) {
      let lamports = await this.#conn.getMinimumBalanceForRentExemption(PROFILE_ACCOUNT_LEN);

      // Construct ix
      const create_profile_account_ix = SystemProgram.createAccountWithSeed({
        fromPubkey: payerKey,
        newAccountPubkey: profileKey,
        basePubkey: payerKey,
        seed: PLAYER_PROFILE_SEED,
        lamports: lamports,
        space: PROFILE_ACCOUNT_LEN,
        programId: PROGRAM_ID,
      });
      tx.add(create_profile_account_ix);
    } else {
      throw new Error('Profile account already exists: ');
    }

    // Get pfp ready
    const pfpKey = pfp === undefined ? PublicKey.default : new PublicKey(pfp);

    // Construct ix
    const create_profile_ix = intruction.createPlayerProfile(payerKey, profileKey, nick, pfpKey);

    tx.add(create_profile_ix);

    await wallet.sendTransaction(tx);

    return profileKey.toBase58();
  }

  async createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<string> {
    return '';
  }

  async registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void> {}

  async unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void> {}

  async getGameAccount(addr: string): Promise<GameAccount | undefined> {
    return undefined;
  }

  async getGameBundle(addr: string): Promise<GameBundle | undefined> {
    return undefined;
  }

  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    const conn = this.#conn;
    const profileKey = new PublicKey(addr);

    const profileAccount = await conn.getAccountInfo(profileKey);
    if (profileAccount) {
      const profileAccountData = profileAccount.data;
      const { nick, pfp } = PlayerState.deserialize(profileAccountData);
      if (pfp !== undefined) {
        return { addr: addr, nick: nick, pfp: pfp.toBase58() };
      } else {
        return { addr: addr, nick: nick, pfp: undefined }
      }
    } else {
      return undefined;
    }
  }

  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    return undefined;
  }

  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    return undefined;
  }
}

export default SolanaTransport;
