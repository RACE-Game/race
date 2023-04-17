import {
  SystemProgram,
  Connection,
  PublicKey,
  Transaction,
  clusterApiUrl,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { serialize } from '@dao-xyz/borsh';
import os from 'os';
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
import { CreatePlayerProfile } from './instruction';

const PLAYER_PROFILE_SEED = 'race-player-1000';
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
    const payerPublicKey = new PublicKey(payer);

    const profileAccountPublicKey = await PublicKey.createWithSeed(payerPublicKey, PLAYER_PROFILE_SEED, PROGRAM_ID);

    console.log('Player profile public key: ', profileAccountPublicKey);
    let tx = new Transaction();

    // Check if player account already exists
    if (!(await this.#conn.getAccountInfo(profileAccountPublicKey))) {
      console.log('Profile account already exists: ', profileAccountPublicKey);
      // FIXME: replace dataLength
      let lamports = await this.#conn.getMinimumBalanceForRentExemption(dataLength);

      const create_profile_account_ix = SystemProgram.createAccountWithSeed({
        fromPubkey: payerPublicKey,
        newAccountPubkey: profileAccountPublicKey,
        basePubkey: payerPublicKey,
        seed: PLAYER_PROFILE_SEED,
        lamports: lamports,
        space: dataLength,
        programId: PROGRAM_ID,
      });

      tx.add(create_profile_account_ix);
    }

    // Get pfp ready
    const pfpPublicKey = pfp === undefined ? PublicKey.default : new PublicKey(pfp);
    const ix_data = serialize(new CreatePlayerProfile(nick));
    const init_profile_ix = new TransactionInstruction({
      keys: [
        {
          pubkey: payerPublicKey,
          isSigner: true,
          isWritable: false,
        },
        {
          pubkey: profileAccountPublicKey,
          isSigner: true,
          isWritable: false,
        },
        {
          pubkey: pfpPublicKey,
          isSigner: false,
          isWritable: false,
        },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(ix_data),
    });

    tx.add(init_profile_ix);

    wallet.sendTransaction(tx);

    return '';
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
    return undefined;
  }

  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    return undefined;
  }

  async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    return undefined;
  }
}

export default SolanaTransport;
