import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance } from "@race-foundation/sdk-core";
import { Chain } from './common'
import { Balance, getFullnodeUrl, SuiClient, SuiTransactionBlock } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions'
import { bcs } from '@mysten/bcs';
import { SuiWallet } from "./sui-wallet";
import { LocalSuiWallet } from "./local-wallet";
import { PACKAGE_ID, PROFILE_TABLE_ID } from './constants'
import { JsonU64 } from "@mysten/sui/dist/cjs/transactions/data/internal";
import { ok } from "assert";
import { promises } from "dns";
// import { getFaucetHost, requestSuiFromFaucetV0 } from '@mysten/sui/faucet';
export interface ISigner {
  send(tx: Transaction, client: SuiClient) : Promise<string>
}
// import { TransactionBlock } from '@mysten/sui.js';
export class SuiTransport implements ITransport {

  suiClient: SuiClient

  constructor(url: string) {
    // For devnet 'https://fullnode.devnet.sui.io:443'
    this.suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });
  }

  get chain(): Chain { return 'sui' }
  // debug used , mast be removed in production
  // getKeypair() {
  //   // const keypair = Ed25519Keypair.generate();
  //   // console.log('====>',keypair.getSecretKey())
  //   // 0xca573b8fd1fc4478010270f0f9099392925f5c116c7c12a71e980e43199c5327
  //   // suiprivkey1qpvr0xrc7hlzjdz39mrvgqptj30vkqdwwh43kedzp22s9na47vtlg7z4dpj
  //   const keypair = Ed25519Keypair.fromSecretKey('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2')
  //   // const keypair = Ed25519Keypair.deriveKeypairFromSeed('0xd5d985d94c233d39c7578dc7b4db6e03fc9a661c7de2710d0144fc855fd7973c')
  //   const address = keypair.getPublicKey().toSuiAddress()
  //   const gameModuleAddress = PACKAGE_ID;
  //   const gasBudget = 5_000_001
  //   console.log(`Keypair: ${address + '-' + keypair.getSecretKey()}`);
  //   return { keypair, address, gameModuleAddress, gasBudget }
  // }
  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams, resp: ResponseHandle<CreateGameResponse, CreateGameError>): Promise<void> {
    const suiClient = this.suiClient
    // const info = this.getKeypair()
    const transaction = new Transaction();;
    console.log('=====>createGameAccount wallet', wallet)
    console.log('=====>createGameAccount params', params)
    let args = [
      transaction.pure.string(params.title), // title string
      transaction.pure.option('address', params.bundleAddr), // bundle_addr address params
      transaction.pure.option('address', wallet.walletAddr), // owner address wallet
      transaction.pure.option('address', null), // recipient_addr address params
      transaction.pure.option('address', params.tokenAddr), // token_addr address params "0x2"
      transaction.pure.u64(params.maxPlayers), // max_players u64 params
      transaction.pure.u32(2), // data_len u32 params
      transaction.pure.vector('u8', params.data), // data vector<u8> params
    ]
    let gameType = ''
    const kind = params.entryType.kind
    switch (kind) {
      case 'cash':
        gameType = 'create_cash_game'
        args = [
          ...args,
          transaction.pure.option('address', null), // min_deposit u64 params
          transaction.pure.option('address', null), // max_deposit u64 params
        ]
        break;
      case 'ticket':
        gameType = 'create_ticket_game'
        args = [
          ...args,
          transaction.pure.option('address', null), // amount u64 params
        ]
        break;
      case 'gating':
        gameType = 'create_gating_game'
        args = [
          ...args,
          transaction.pure.string('yuumi Ganme'), // collection String params
        ]
        break;
      default:
    }
    transaction.moveCall({
      target: `${PACKAGE_ID}::game::${gameType}`,
      arguments: args,
    });
    transaction.setGasBudget(5_000_001);
    try {
      // suiCLient fucntion
      // const result = await suiClient.signAndExecuteTransaction({
      //   transaction: transaction,
      //   signer: info.keypair,
      //   requestType: 'WaitForLocalExecution',
      //   options: {
      //     showEffects: true,
      //   },
      // });
      // wallet function
      const result = wallet.signAndExecuteTransaction(transaction, suiClient)
      console.log('Transaction Result:', result);
      // return { ok: 'ok' }

    } catch (error) {
      console.log('===>targe 22 t', `${PACKAGE_ID}::game::${gameType}`,)
      console.error('Error while creating game account:', error);
      // return { err: 'err'}
    }
  }

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams, resp: ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>): Promise<void> {
    const suiClient = this.suiClient;
    // const info = this.getKeypair()
    const createPlayeAccount = async () => {
      const transaction = new Transaction();
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
      transaction.moveCall({
        target: `${PACKAGE_ID}::profile::create_profile`,
        // arguments: [serializedOption, bcs.option(bcs.string()).serialize(undefined)],
        arguments: [
          transaction.pure.string('yuumi'),
          transaction.pure.option('address', null),
          transaction.object(PROFILE_TABLE_ID),
        ],
      });

      transaction.setGasBudget(5_000_001);

      try {
        const result = wallet.wallet(transaction, suiClient)
        // suiCLient fucntion
        // const result = await suiClient.signAndExecuteTransaction({
        //   transaction: transaction,
        //   signer: info.keypair,
        //   requestType: 'WaitForLocalExecution',
        //   options: {
        //     showEffects: true,
        //   },
        // });
        console.log('Transaction Result:', result);
        return { result: 'ok' }

      } catch (error) {
        console.error('Error while creating game account:', error);
        return { result: 'err' }
      }
    };
    createPlayeAccount();
    // return { result: 'ok' }
  }
  // async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<SendTransactionResult> {
  //   const w = (wallet as SuiWallet)
  //   const suiClient = this.suiClient
  //   const createPlayeAccount = async () => {
  //     const transaction = new Transaction();
  //     // for debugging only 
  //     const gameModuleAddress = '0x195ff9c5fe7c49a1695ce2cab6bf72e109208203b129f60fa880327a22d5e48d';
  //     const PROFILE_TABLE_ID = '0xcf7ae3a5c7e16ec9cc964d998ebadcaf623c61a6c945c3069498c71484ebfc1f';
  //     // For debugging only
  //     try {
  //       const object = await suiClient.getObject({
  //         id: PROFILE_TABLE_ID,
  //         options: { showContent: true }
  //       });
  //       console.log('Profile table:', object);
  //     } catch (error) {
  //       console.error('Error while accessing profile table:', error);
  //     }
  //     const createGameAccountFunction = 'create_profile';
  //     transaction.moveCall({
  //       target: `${gameModuleAddress}::profile::${createGameAccountFunction}`,
  //       // arguments: [serializedOption, bcs.option(bcs.string()).serialize(undefined)],
  //       arguments: [
  //         transaction.pure.string(params.nick),
  //         transaction.pure.option('address', params.pfp || null),
  //         transaction.object(PROFILE_TABLE_ID),
  //       ],
  //     });

  //     transaction.setGasBudget(5_000_001);

  //     try {
  //       const result = w.signAndExecuteTransaction(transaction, suiClient)
  //       console.log('Transaction Result:', result);
  //       return { result: 'ok' }

  //     } catch (error) {
  //       console.error('Error while creating game account:', error);
  //       return { result: 'err' }
  //     }
  //   };
  //   createPlayeAccount();
  //   return { result: 'ok' }
  // }

  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    try {
      const suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });
      const objectResponse = await suiClient.getObject({
        id: PROFILE_TABLE_ID,
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
  closeGameAccount(wallet: IWallet, params: CloseGameAccountParams, resp: ResponseHandle): Promise<void> {
    throw new Error("Method not implemented.");
  }
  join(wallet: IWallet, params: JoinParams, resp: ResponseHandle<JoinResponse, JoinError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  deposit(wallet: IWallet, params: DepositParams, resp: ResponseHandle<DepositResponse, DepositError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  createRecipient(wallet: IWallet, params: CreateRecipientParams, resp: ResponseHandle<CreateRecipientResponse, CreateRecipientError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  registerGame(wallet: IWallet, params: RegisterGameParams, resp: ResponseHandle<RegisterGameResponse, RegisterGameError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  unregisterGame(wallet: IWallet, params: UnregisterGameParams, resp: ResponseHandle): Promise<void> {
    throw new Error("Method not implemented.");
  }
  getGameAccount(addr: string): Promise<GameAccount | undefined> {
    throw new Error("Method not implemented.");
  }
  getGameBundle(addr: string): Promise<GameBundle | undefined> {
    throw new Error("Method not implemented.");
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
  getTokenDecimals(addr: string): Promise<number | undefined> {
    throw new Error("Method not implemented.");
  }
  getToken(addr: string): Promise<Token | undefined> {
    throw new Error("Method not implemented.");
  }
  getNft(addr: string): Promise<Nft | undefined> {
    throw new Error("Method not implemented.");
  }
  listTokens(tokenAddrs: string[]): Promise<Token[]> {
    throw new Error("Method not implemented.");
  }
  listTokensWithBalance(walletAddr: string, tokenAddrs: string[], storage?: IStorage): Promise<TokenWithBalance[]> {
    throw new Error("Method not implemented.");
  }
  listNfts(walletAddr: string): Promise<Nft[]> {
    throw new Error("Method not implemented.");
  }
  recipientClaim(wallet: IWallet, params: RecipientClaimParams, resp: ResponseHandle<RecipientClaimResponse, RecipientClaimError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
}

// const wallet = new LocalSuiWallet()

// const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443')
// suiTransport.createPlayerProfile(wallet, {
//   nick: 'yuumi',
// }).then(result => {
//   console.log('createPlayerProfile success:', result)
// })
//   .catch(error => {
//     console.error('getPlayerProfile err:', error);
//   });

// const addr = '0x9d019a5566152d64686d6dedc03740d912dd91dba9ff2089d853929652d4d194'
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

// suiTransport.createGameAccount().then(result => {
//   console.log('createGameAccount success:', result)
// })
//   .catch(error => {
//     console.error('createGameAccount err:', error);
//   });