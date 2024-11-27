import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance, Result } from "@race-foundation/sdk-core";
import { Chain } from './common'
import { Balance, getFullnodeUrl, SuiClient, SuiObjectChange, SuiObjectChangeCreated, SuiTransactionBlock } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions'
import { bcs } from '@mysten/bcs';
import { SuiWallet } from "./sui-wallet";
import { LocalSuiWallet } from "./local-wallet";
import { GAME_OBJECT_TYPE, GAS_BUDGET, MAXIMUM_TITLE_LENGTH, PACKAGE_ID, PROFILE_TABLE_ID, SUI_ICON_URL } from './constants'
import { ISigner, TxResult } from "./signer";


function coerceWallet(wallet: IWallet): asserts wallet is ISigner {
  if (!(wallet instanceof LocalSuiWallet) && !(wallet instanceof SuiWallet)) {
    throw new Error('Invalid wallet instance passed')
  }
}

export class SuiTransport implements ITransport {

  suiClient: SuiClient

  constructor(url: string) {
    this.suiClient = new SuiClient({ url });
  }

  get chain(): Chain { return 'sui' }

  async createGameAccount(wallet: IWallet, params: CreateGameAccountParams, resp: ResponseHandle<CreateGameResponse, CreateGameError>): Promise<void> {

    if (params.title.length > MAXIMUM_TITLE_LENGTH) {
      return resp.failed('invalid-title')
    }

    coerceWallet(wallet)

    const suiClient = this.suiClient
    const transaction = new Transaction();;
    console.log('=====>createGameAccount wallet', wallet)
    console.log('=====>createGameAccount params', params)
    let args = [
      transaction.pure.string(params.title), // title string
      transaction.pure.address(params.bundleAddr), // bundle_addr address params
      transaction.pure.address(wallet.walletAddr), // owner address wallet
      transaction.pure.address(randomPublicKey()), // recipient_addr address params
      transaction.pure.address(params.tokenAddr), // token_addr address params "0x2"
      transaction.pure.u64(params.maxPlayers), // max_players u64 params
      transaction.pure.u32(params.data.length), // data_len u32 params
      transaction.pure.vector('u8', params.data), // data vector<u8> params
    ]
    let entryFunction = ''
    const kind = params.entryType.kind
    switch (kind) {
      case 'cash':
        if (params.entryType.maxDeposit < params.entryType.minDeposit || params.entryType.minDeposit < 0) {
          return resp.failed('invalid-depsoit-range')
        }
        entryFunction = 'create_cash_game'
        args = [
          ...args,
          transaction.pure.u64(params.entryType.minDeposit), // min_deposit u64 params
          transaction.pure.u64(params.entryType.maxDeposit), // max_deposit u64 params
        ]
        break;
      case 'ticket':
        entryFunction = 'create_ticket_game'
        args = [
          ...args,
          transaction.pure.u64(params.entryType.amount), // amount u64 params
        ]
        break;
      case 'gating':
        entryFunction = 'create_gating_game'
        args = [
          ...args,
          transaction.pure.string(params.entryType.collection), // collection String params
        ]
        break;
    }
    transaction.moveCall({
      target: `${PACKAGE_ID}::game::${entryFunction}`,
      arguments: args,
    });

    const result = await wallet.send(transaction, suiClient, resp)

    if ("err" in result) {
      return resp.transactionFailed(result.err)
    }

    const objectChange = resolveObjectCreatedByType(result.ok, GAME_OBJECT_TYPE, resp)
    if (objectChange === undefined) return;

    console.log('Transaction Result:', objectChange);
    return resp.succeed({
      gameAddr: objectChange.objectId,
      signature: result.ok.digest,
    })
  }

  async createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams, resp: ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>): Promise<void> {
    coerceWallet(wallet)

    const suiClient = this.suiClient;

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
      arguments: [
        transaction.pure.string(params.nick),
        transaction.pure.option('address', params.pfp),
        transaction.object(PROFILE_TABLE_ID),
      ],
    });

    const result = await wallet.send(transaction, suiClient, resp)
    if ("err" in result) {
      return resp.transactionFailed(result.err)
    }

    const objectChange = resolveObjectCreatedByType(result.ok, GAME_OBJECT_TYPE, resp)
    if (objectChange === undefined) return;

    console.log('Transaction Result:', objectChange);
  }
  async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
    try {
      const suiClient = this.suiClient;
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
  async getTokenDecimals(addr: string): Promise<number | undefined> {
    return this.getToken(addr).then(token => token?.decimals);
  }
  async getToken(addr: string): Promise<Token | undefined> {
    const suiClient = this.suiClient;
    const tokenMetadata = await suiClient.getCoinMetadata({ coinType: addr });
    if (!tokenMetadata) return undefined
    const token: Token = {
      addr: addr,
      icon: tokenMetadata.iconUrl || SUI_ICON_URL,
      name: tokenMetadata.name,
      symbol: tokenMetadata.symbol,
      decimals: tokenMetadata.decimals
    }
    console.log('Token:', token);
    return token;
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


function resolveObjectCreatedByType<T, E>(result: TxResult, objectType: string, resp: ResponseHandle<T, E>): SuiObjectChangeCreated | undefined {
  if (!("objectChanges" in result)) {
    resp.transactionFailed('Object changes not found in transaction result')
    return undefined
  }

  const objectChange = result.objectChanges?.find(c => c.type == 'created' && c.objectType == objectType)
  if (objectChange === undefined || objectChange.type !== 'created') {
    resp.transactionFailed('Game object is missing')
    return undefined
  }

  return objectChange
}

function randomPublicKey(): string {
  return Ed25519Keypair.generate().getPublicKey().toSuiAddress()
}
