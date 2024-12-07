import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance, Result, CheckpointOnChain, EntryType, RecipientSlotInit } from "@race-foundation/sdk-core";
import { Chain } from './common'
import { Balance, getFullnodeUrl, MoveStruct, MoveVariant, SuiClient, SuiMoveObject, SuiObjectChange, SuiObjectChangeCreated, SuiObjectResponse, SuiTransactionBlock } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions'
import { bcs } from '@mysten/bcs';
import { SuiWallet } from "./sui-wallet";
import { LocalSuiWallet } from "./local-wallet";
import { GAME_OBJECT_TYPE, GAS_BUDGET, MAXIMUM_TITLE_LENGTH, PACKAGE_ID, PROFILE_STRUCT_TYPE, PROFILE_TABLE_ID, SUI_ICON_URL } from './constants'
import { ISigner, TxResult } from "./signer";
import { option } from "@race-foundation/borsh";


function coerceWallet(wallet: IWallet): asserts wallet is ISigner {
  if (!(wallet instanceof LocalSuiWallet) && !(wallet instanceof SuiWallet)) {
    throw new Error('Invalid wallet instance passed')
  }
}

export class SuiTransport implements ITransport {

  suiClient: SuiClient

  constructor(url: string) {
    console.log('SuiTransport', url)
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
    let args = [
      transaction.pure.string(params.title), // title string
      transaction.pure.address(params.bundleAddr), // bundle_addr address params
      transaction.pure.address(wallet.walletAddr), // owner address wallet
      transaction.pure.address(randomPublicKey()), // recipient_addr address params
      transaction.pure.address(params.tokenAddr), // token_addr address params "0x2"
      transaction.pure.u16(params.maxPlayers), // max_players u64 params
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
  // todo contract
  join(wallet: IWallet, params: JoinParams, resp: ResponseHandle<JoinResponse, JoinError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  // todo contract
  deposit(wallet: IWallet, params: DepositParams, resp: ResponseHandle<DepositResponse, DepositError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  async createRecipient(wallet: IWallet, params: CreateRecipientParams, resp: ResponseHandle<CreateRecipientResponse, CreateRecipientError>): Promise<void> {
    const transaction = new Transaction();
    const suiClient = this.suiClient;
    coerceWallet(wallet)
    let builder = transaction.moveCall({
      target: `${PACKAGE_ID}::recipient::new_recipient_builder`,
    });
    params.slots.forEach((slot: RecipientSlotInit) => {
      let slotType: number = 0
      if (slot.slotType === 'nft') {
        slotType = 0
      } else if (slot.slotType === 'token') {
        slotType = 1
      }
      // Struct
      const addrStruct = bcs.struct('AddrStruct', {
        owner: bcs.struct('Owner', { addr: bcs.string()}),
        weights: bcs.u64()
      })
      // Struct
      const identifierStruct = bcs.struct('AdentifierStruct', {
        owner: bcs.struct('Owner', { identifier: bcs.string()}),
        weights: bcs.u64()
      })
      let shares
      if( 'addr' in slot.initShares) {
        shares = addrStruct.serialize({ owner: { addr: '0x'}, weights: 20 });
      } else {
        shares = identifierStruct.serialize({ owner: { identifier: '0x'}, weights: 20 });
      }
      builder = transaction.moveCall({
        target: `${PACKAGE_ID}::recipient::create_recipient_slot`,
        arguments: [
          transaction.pure.u8(slot.id), // id u8
          transaction.pure.option('address', slot.tokenAddr),
          bcs.vector(bcs.u8()).serialize([slotType]),
          shares,
          builder,
        ]
      })
    })
    let recipient = transaction.moveCall({
      target: `${PACKAGE_ID}::recipient::create_recipient`,
      arguments: [
        transaction.pure.option('address', wallet.walletAddr),
        builder,
      ]
    });
    console.log('createRecipientResult', recipient)
    const result = await wallet.send(transaction, suiClient, resp)
    if ("err" in result) {
      return resp.transactionFailed(result.err)
    }
    const objectChange = resolveObjectCreatedByType(result.ok, GAME_OBJECT_TYPE, resp)
    if (objectChange === undefined) return;

    throw new Error("Method not implemented.");
  }
  // todo contract
  registerGame(wallet: IWallet, params: RegisterGameParams, resp: ResponseHandle<RegisterGameResponse, RegisterGameError>): Promise<void> {
    throw new Error("Method not implemented.");
  }
  // todo contract
  unregisterGame(wallet: IWallet, params: UnregisterGameParams, resp: ResponseHandle): Promise<void> {
    throw new Error("Method not implemented.");
  }
  // todo sui
  async getGameAccount(addr: string): Promise<GameAccount | undefined> {
    const suiClient = this.suiClient;
    const info: SuiObjectResponse = await suiClient.getObject({
      id: addr,
      options: {
        showContent: true,
        showType: true
      }
    })
    const content = info.data?.content;
    if (!content || content.dataType !== 'moveObject') {
      return undefined;
    }
    if (!content.fields) return undefined;
    let fields: MoveStruct = content.fields
    if (Array.isArray(fields)) { return undefined }
    if ('fields' in fields) { return undefined }
    return {
      addr: addr,
      title: fields?.title as string,
      bundleAddr: fields.bundle_addr as string,
      tokenAddr: fields.token_addr as string,
      ownerAddr: fields.owner as string,
      settleVersion: BigInt(fields.settle_version?.toString() || 0),
      accessVersion: BigInt(fields.access_version?.toString() || 0),
      players: fields.players as [],
      deposits: fields.deposits as [],
      servers: fields.servers as [],
      transactorAddr: fields.transactor_addr as string | undefined,
      votes: fields.votes as [],
      unlockTime: BigInt(fields.unlock_time?.toString() || 0),
      maxPlayers: Number(fields.max_players) || 0,
      dataLen: Number(fields.data_len) || 0,
      data: fields.data ? new Uint8Array(fields.data as number[]) : new Uint8Array(),
      entryType: fields?.entry_type ? (fields.entry_type as MoveVariant).variant as unknown as EntryType : 'None' as unknown as EntryType,
      recipientAddr: fields.recipient_addr as string,
      checkpointOnChain: fields.checkpoint as unknown as CheckpointOnChain | undefined,
      entryLock: fields.entry_lock ? (fields.entry_lock as MoveVariant).variant as 'Closed' | 'Open' | 'JoinOnly' | 'DepositOnly' : 'Closed',
    }
  }
  // todo sui and contract
  getGameBundle(addr: string): Promise<GameBundle | undefined> {
    throw new Error("Method not implemented.");
  }
  // todo 
  async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
    const suiClient = this.suiClient;
    const info: SuiObjectResponse = await suiClient.getObject({
      id: addr,
      options: {
        showContent: true,
        showType: true
      }
    })
    const content = info;
    console.log('content:', content)
    throw new Error("Method not implemented.");
  }
  getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
    // ObjectID 
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
    return token;
  }

  async getNft(addr: string): Promise<Nft | undefined> {
    const suiClient = this.suiClient;
    const objectResponse: SuiObjectResponse = await suiClient.getObject({
      id: addr,
      options: {
        showContent: true,
        showType: true
      }
    })
    if (objectResponse.error) {
      console.error('Error fetching NFT:', objectResponse.error);
      return undefined
    }
    const content = objectResponse.data?.content;
    if (!content || content.dataType !== 'moveObject') {
      return undefined;
    }
    if (!content.fields) return undefined;
    let fields = content.fields
    if (Array.isArray(fields)) { return undefined }
    if ('fields' in fields) { return undefined }
    if ('balance' in fields) return undefined;
    if (fields["image_url"] || fields["img_url"]) {
      return {
        addr: addr,
        image: fields?.image_url?.toString() || fields?.img_url?.toString() || '',
        name: fields.name?.toString() || '',
        symbol: fields.symbol?.toString() || fields.name?.toString() || '',
        collection: objectResponse?.data?.type || undefined,
        metadata: objectResponse
      }
    }
    return undefined
  }
  async listTokens(tokenAddrs: string[]): Promise<Token[]> {
    const promises = tokenAddrs.map(async addr => {
      return await this.getToken(addr)
    })
    let tokens = await (await Promise.all(promises)).filter((t): t is Token => t !== undefined)
    return tokens
  }
  async listTokensWithBalance(walletAddr: string, tokenAddrs: string[], storage?: IStorage): Promise<TokenWithBalance[]> {
    const that = this
    const suiClient = this.suiClient;
    const tokenMetadata = await suiClient.getOwnedObjects({ owner: walletAddr });
    if (!tokenMetadata) {
      console.error('Error fetching token metadata:', tokenMetadata)
      return []
    }
    let ids = tokenMetadata.data.map((obj: SuiObjectResponse) => obj?.data?.objectId)
      .filter((id): id is string => id !== undefined)
    const objectResponses: SuiObjectResponse[] = await suiClient.multiGetObjects({
      ids,
      options: {
        showContent: true,
        showType: true
      }
    })
    const tokenAndBalances: (TokenWithBalance | undefined)[] = await Promise.all(
      objectResponses.map(async (obj) => {
        try {
          const content = obj.data?.content;
          if (!content || content.dataType !== 'moveObject' || !content.fields) {
            return undefined;
          }
          const fields = content.fields;
          if (Array.isArray(fields) || 'fields' in fields || !('balance' in fields)) {
            return undefined;
          }

          const list = obj.data?.type?.match(/0x2::coin::Coin<(.+)>/);
          if (!list) return undefined;

          const objType = list[1];
          if (!tokenAddrs.includes(objType)) return undefined;

          const token = await that.getToken(objType);
          if (!token) return undefined;

          return {
            addr: objType,
            icon: token.icon,
            name: token.name,
            symbol: token.symbol,
            decimals: token.decimals,
            amount: BigInt(fields.balance?.toString() || 0),
            uiAmount: fields.balance?.toString() || '',
          };
        } catch (error) {
          console.error('Error processing object:', error);
          return undefined;
        }
      })
    );
    const multokenAndBalances: TokenWithBalance[] = tokenAndBalances
      .filter((t: TokenWithBalance | undefined): t is TokenWithBalance => t !== undefined)
    const mergedData = multokenAndBalances.reduce((acc: any, obj: TokenWithBalance) => {

      if (acc[obj.addr]) {
        acc[obj.addr].amount += obj.amount;
      } else {
        acc[obj.addr] = { addr: obj.addr, amount: obj.amount };
      }
      acc[obj.addr].addr = obj.addr;
      acc[obj.addr].icon = obj.icon;
      acc[obj.addr].name = obj.name;
      acc[obj.addr].symbol = obj.symbol;
      acc[obj.addr].decimals = obj.decimals;
      acc[obj.addr].uiAmount = obj.uiAmount;
      return acc;
    }, {});

    return Object.values(mergedData)
  }

  async listNfts(walletAddr: string): Promise<Nft[]> {
    const suiClient = this.suiClient;
    const tokenMetadata = await suiClient.getOwnedObjects({ owner: walletAddr });
    if (!tokenMetadata) return []
    let ids = tokenMetadata.data.map((obj) => obj?.data?.objectId)
      .filter((id): id is string => id !== undefined)
    const objectResponses: SuiObjectResponse[] = await suiClient.multiGetObjects({
      ids,
      options: {
        showContent: true,
        showType: true
      }
    })
    let nfts = objectResponses
      .map((obj: SuiObjectResponse) => {
        const content = obj.data?.content;
        if (!content || content.dataType !== 'moveObject') {
          return undefined;
        }
        if (!content.fields) return undefined;
        let fields = content.fields
        if (Array.isArray(fields)) { return undefined }
        if ('fields' in fields) { return undefined }
        if ('balance' in fields) return undefined;
        if (fields["image_url"] || fields["img_url"]) {
          return {
            addr: walletAddr,
            image: fields?.image_url?.toString() || fields?.img_url?.toString() || '',
            name: fields.name?.toString() || '',
            symbol: fields.symbol?.toString() || fields.name?.toString() || '',
            collection: obj?.data?.type || undefined,
            metadata: obj
          }
        }
        return undefined
      })
      .filter((obj: Nft | undefined): obj is Nft => obj !== undefined)
    return nfts
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
