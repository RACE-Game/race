import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, INft, IStorage, IToken, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, TransactionResult, UnregisterGameParams, VoteParams } from "@race-foundation/sdk-core";
import { Chain } from './common'
import { Balance, getFullnodeUrl, SuiClient, SuiTransactionBlock } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions'
import { bcs } from '@mysten/bcs';
import { SuiWallet } from "./sui-wallet";
import { LocalSuiWallet } from "./local-wallet";
import { value } from "valibot";
// import { getFaucetHost, requestSuiFromFaucetV0 } from '@mysten/sui/faucet';

// import { TransactionBlock } from '@mysten/sui.js';
export class SuiTransport implements ITransport {

  suiClient: SuiClient

  constructor(url: string) {
    // For devnet 'https://fullnode.devnet.sui.io:443'
    this.suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });
  }

  get chain(): Chain { return 'sui' }

  async createGameAccount(wallet?: IWallet, params?: CreateGameAccountParams): Promise<TransactionResult<string>> {
    throw new Error("Method not implemented.");
  }
  closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }
  join(wallet: IWallet, params: JoinParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }
  deposit(wallet: IWallet, params: DepositParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }
  vote(wallet: IWallet, params: VoteParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<TransactionResult<void>> {
    const w = (wallet as SuiWallet)

    const suiClient = this.suiClient

    // create Ed25519 keypair
    // const keypair = Ed25519Keypair.generate();
    // const address = keypair.getPublicKey().toSuiAddress();
    // debug used , mast be removed in production
    // 0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e
    // suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2
    // const keypair = Ed25519Keypair.fromSecretKey('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2')
    // const address = keypair.getPublicKey().toSuiAddress()
    // console.log(`New address: ${address + '-' + keypair.getSecretKey()}`);
    const createGameAccount = async () => {
      const transaction = new Transaction();
      // for debugging only 
      const gameModuleAddress = '0x195ff9c5fe7c49a1695ce2cab6bf72e109208203b129f60fa880327a22d5e48d';
      const PROFILE_TABLE_ID = '0xcf7ae3a5c7e16ec9cc964d998ebadcaf623c61a6c945c3069498c71484ebfc1f';
      // For debugging only
      try {
        const object = await suiClient.getObject({
          id: PROFILE_TABLE_ID,
          options: { showContent: true }
        });
        console.log('Profile table:', object);
      } catch (error) {
        console.error('Error while accessing profile table:', error);
      }
      const createGameAccountFunction = 'create_profile';
      transaction.moveCall({
        target: `${gameModuleAddress}::profile::${createGameAccountFunction}`,
        // arguments: [serializedOption, bcs.option(bcs.string()).serialize(undefined)],
        arguments: [
          transaction.pure.string(params.nick),
          transaction.pure.option('address', params.pfp || null),
          transaction.object(PROFILE_TABLE_ID),
        ],
      });

      transaction.setGasBudget(5_000_001);

      try {
        // suiCLient fucntion
        // const result = await suiClient.signAndExecuteTransaction({
        //   transaction: transaction,
        //   signer: keypair,
        //   requestType: 'WaitForLocalExecution',
        //   options: {
        //     showEffects: true,
        //   },
        // });
        // wallet function
        const result = w.signAndExecuteTransaction(transaction, suiClient)
        console.log('Transaction Result:', result);
        return { result: 'ok' }

      } catch (error) {
        console.error('Error while creating game account:', error);
        return { result: 'err' }
      }
    };
    createGameAccount();
    return { result: 'ok' }
  }
  publishGame(wallet: IWallet, params: PublishGameParams): Promise<TransactionResult<string>> {
    throw new Error("Method not implemented.");
  }
  createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<TransactionResult<string>> {
    throw new Error("Method not implemented.");
  }
  registerGame(wallet: IWallet, params: RegisterGameParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }
  unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }
  getGameAccount(addr: string): Promise<GameAccount | undefined> {
    throw new Error("Method not implemented.");
  }
  getGameBundle(addr: string): Promise<GameBundle | undefined> {
    throw new Error("Method not implemented.");
  }
  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    try {
      const suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });
      const objectResponse = await suiClient.getObject({
        id: '0xcf7ae3a5c7e16ec9cc964d998ebadcaf623c61a6c945c3069498c71484ebfc1f',
        options: {
          showContent: true,
          showType: true,
        }
      });

      if (!objectResponse.data) {
        return undefined;
      }
      // Extract the content from the response
      const content = objectResponse.data.content;
      console.log('>objectResponse:', objectResponse)
      if (!content || content.dataType !== 'moveObject') {
        return undefined;
      }
      // Parse the fields from the content
      const fields = content.fields as any;
      // Convert the SUI object data into PlayerProfile format
      return {
        addr: addr,
        nick: fields.nick,
        pfp: fields.pfp,
        // Add any other fields that are part of your PlayerProfile interface
      };
    } catch (error) {
      console.error('Error fetching player profile:', error);
      return undefined;
    }
  }
  getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    throw new Error("Method not implemented.");
  }
  getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    throw new Error("Method not implemented.");
  }
  getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined> {
    throw new Error("Method not implemented.");
  }
  getRecipient(addr: string): Promise<RecipientAccount | undefined> {
    throw new Error("Method not implemented.");
  }
  getToken(addr: string): Promise<IToken | undefined> {
    throw new Error("Method not implemented.");
  }
  getNft(addr: string, storage?: IStorage): Promise<INft | undefined> {
    throw new Error("Method not implemented.");
  }
  listTokens(storage?: IStorage): Promise<IToken[]> {
    throw new Error("Method not implemented.");
  }
  listNfts(walletAddr: string, storage?: IStorage): Promise<INft[]> {
    throw new Error("Method not implemented.");
  }
  fetchBalances(walletAddr: string, tokenAddrs: string[]): Promise<Map<string, bigint>> {
    throw new Error("Method not implemented.");
  }
  recipientClaim(wallet: IWallet, params: RecipientClaimParams): Promise<TransactionResult<void>> {
    throw new Error("Method not implemented.");
  }

}

const wallet = new LocalSuiWallet()

const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443')
suiTransport.createPlayerProfile(wallet, {
  nick: 'yuumi',
}).then(result => {
  console.log('createPlayerProfile success:', result)
})
  .catch(error => {
    console.error('getPlayerProfile err:', error);
  });

//   const addr = '0x9d019a5566152d64686d6dedc03740d912dd91dba9ff2089d853929652d4d194'
// suiTransport.getPlayerProfile(addr).then((result: any) => {
//     console.log('getPlayerProfile:', result)
//     if (result.addr) {
//         console.log('getPlayerProfile success:', result)
//     } else {
//         console.error('getPlayerProfile err:', result.error());
//     }
// })
//     .catch(error => {
//         console.error('getPlayerProfile err:', error);
//     });