import { countCandyMachineV2Items } from '@metaplex-foundation/js';
import { Connection, PublicKey, clusterApiUrl } from '@solana/web3.js';
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

const PLAYER_PROFILE_SEED = "race-0001";
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
    const transport = new SolanaTransport("https://localhost:8899");
    const connection = transport.#conn;

    if (params.nick.length > 16) {
      // FIXME: better error message?
      return 'Player nick name exceeds 16 chars';
    }

    const payerPublickey = wallet.walletAddr;
    const payerPublickKey = new PublicKey(payerPublickey);

    const profileAccountPublicKey = await PublicKey.createWithSeed(payerPublickKey, PLAYER_PROFILE_SEED, PROGRAM_ID);

    console.log("Player profile public key: ", profileAccountPublicKey);

    // Check if player account already exists
    if (await connection.getAccountInfo(profileAccountPublicKey)) {
      console.log("Profile account already exists: ", profileAccountPublicKey);
      return '';
    }

    // Get pfp ready
    if (params.pfp?.length > 0) {
      const pfpPublicKey = new PublicKey(params.pfp?);
    } else {

    }
    // const connection = new Connection(clusterApiUrl(params.url));
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
