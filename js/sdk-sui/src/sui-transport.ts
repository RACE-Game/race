import {
    AttachBonusError,
    AttachBonusParams,
    AttachBonusResponse,
    CloseGameAccountParams,
    CloseGameAccountError,
    CloseGameAccountResponse,
    CreateGameAccountParams,
    CreatePlayerProfileParams,
    CreateRegistrationParams,
    DepositParams,
    GameAccount,
    GameBundle,
    Nft,
    IStorage,
    Token,
    ITransport,
    IWallet,
    JoinParams,
    PlayerProfile,
    PublishGameParams,
    RecipientAccount,
    RecipientClaimParams,
    RegisterGameParams,
    RegistrationAccount,
    RegistrationWithGames,
    ServerAccount,
    SendTransactionResult,
    UnregisterGameParams,
    VoteParams,
    ResponseHandle,
    CreateGameResponse,
    CreateGameError,
    CreatePlayerProfileError,
    CreatePlayerProfileResponse,
    CreateRecipientError,
    CreateRecipientParams,
    CreateRecipientResponse,
    DepositError,
    DepositResponse,
    JoinError,
    JoinResponse,
    RecipientClaimError,
    RecipientClaimResponse,
    RegisterGameError,
    RegisterGameResponse,
    TokenBalance,
    Result,
    CheckpointOnChain,
    EntryType,
    RecipientSlotInit,
    RecipientSlotShareInit,
} from '@race-foundation/sdk-core'
import { Chain } from './common'
import {
    Balance,
    getFullnodeUrl,
    MoveStruct,
    MoveVariant,
    ObjectOwner,
    PaginatedObjectsResponse,
    RawData,
    SuiClient,
    SuiMoveObject,
    SuiObjectChange,
    SuiObjectChangeCreated,
    SuiObjectData,
    SuiObjectResponse,
    SuiTransactionBlock,
} from '@mysten/sui/client'
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'
import { Transaction, TransactionObjectArgument } from '@mysten/sui/transactions'
import { bcs, BcsType, fromBase64 } from '@mysten/bcs'
import {
    Parser,
    GameAccountParser,
    GameBundleParser,
    PlayerPorfileParser,
    RegistrationAccountParser,
    RecipientAccountParser,
    ServerParser
} from './types'
import { SuiWallet } from './sui-wallet'
import { LocalSuiWallet } from './local-wallet'
import {
    CLOCK_ID,
    GAME_OBJECT_TYPE,
    MAXIMUM_TITLE_LENGTH,
    PACKAGE_ID,
    SUI_ICON_URL,
} from './constants'
import { ISigner, TxResult } from './signer'

function coerceWallet(wallet: IWallet): asserts wallet is ISigner {
    if (!(wallet instanceof LocalSuiWallet) && !(wallet instanceof SuiWallet)) {
        throw new Error('Invalid wallet instance passed')
    }
}

export class SuiTransport implements ITransport {
    suiClient: SuiClient

    constructor(url: string) {
        console.log('SuiTransport', url)
        this.suiClient = new SuiClient({ url })
    }

    get chain(): Chain {
        return 'sui'
    }

    async createGameAccount(
        wallet: IWallet,
        params: CreateGameAccountParams,
        resp: ResponseHandle<CreateGameResponse, CreateGameError>
    ): Promise<void> {
        if (params.title.length > MAXIMUM_TITLE_LENGTH) {
            return resp.failed('invalid-title')
        }

        coerceWallet(wallet)

        const suiClient = this.suiClient
        const transaction = new Transaction()
        let recipientAddr = ''
        if ('recipientAddr' in params) {
            recipientAddr = params.recipientAddr
        }
        let create_game_args = [
            transaction.pure.string(params.title),
            transaction.pure.address(params.bundleAddr),
            transaction.pure.address(wallet.walletAddr),
            transaction.pure.address(recipientAddr),
            transaction.pure.string(params.tokenAddr),
            transaction.pure.u16(params.maxPlayers),
            transaction.pure.u32(params.data.length),
            transaction.pure.vector('u8', params.data),
        ]
        let entryFunction = ''
        let entry_type_args: any = []
        const kind = params.entryType.kind
        switch (kind) {
            case 'cash':
                if (params.entryType.maxDeposit < params.entryType.minDeposit || params.entryType.minDeposit < 0) {
                    return resp.failed('invalid-depsoit-range')
                }
                entryFunction = 'create_cash_entry'
                entry_type_args = [
                    transaction.pure.u64(params.entryType.minDeposit),
                    transaction.pure.u64(params.entryType.maxDeposit),
                ]
                break
            case 'ticket':
                entryFunction = 'create_ticket_entry'
                entry_type_args = [
                    transaction.pure.u64(params.entryType.amount),
                ]
                break
            case 'gating':
                entryFunction = 'create_gating_entry'
                entry_type_args = [
                    transaction.pure.string(params.entryType.collection),
                ]
                break
            default:
                entryFunction = 'create_disabled_entry'
        }
        let entry_type_result = transaction.moveCall({
            target: `${PACKAGE_ID}::game::${entryFunction}`,
            arguments: entry_type_args,
        })
        transaction.moveCall({
            target: `${PACKAGE_ID}::game::create_game`,
            arguments: [...create_game_args, entry_type_result],
            typeArguments: [params.tokenAddr],
        })

        const result = await wallet.send(transaction, suiClient, resp)

        if ('err' in result) {
            return resp.transactionFailed(result.err)
        }

        const objectChange = resolveObjectCreatedByType(result.ok, GAME_OBJECT_TYPE, resp)
        if (objectChange === undefined) return

        console.log('Transaction Result:', objectChange)
        return resp.succeed({
            gameAddr: objectChange.objectId,
            signature: result.ok.digest,
        })
    }

    async createPlayerProfile(
        wallet: IWallet,
        params: CreatePlayerProfileParams,
        resp: ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>
    ): Promise<void> {
        coerceWallet(wallet)

        const suiClient = this.suiClient


        const exist = await this.getPlayerProfile(wallet.walletAddr)

        if (!exist) {
            throw new Error('Player profile already exists')
        }
        const transaction = new Transaction()
        transaction.moveCall({
            target: `${PACKAGE_ID}::profile::create_profile`,
            arguments: [
                transaction.pure.string(params.nick),
                transaction.pure.option('address', params.pfp),
            ],
        })

        const result = await wallet.send(transaction, suiClient, resp)
        if ('err' in result) {
            return resp.transactionFailed(result.err)
        }

        const objectChange = resolveObjectCreatedByType(result.ok, GAME_OBJECT_TYPE, resp)
        if (objectChange === undefined) return
    }

    async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {

        const resp = await this.suiClient.getOwnedObjects({
            owner: addr,
            filter: {
                StructType: `${PACKAGE_ID}::profile::PlayerProfile`,
            },
            options: {
                showBcs: true,
            },
        })

        return parseObjectData(parseFirstObjectResponse(resp), PlayerPorfileParser)
    }

    closeGameAccount(
        wallet: IWallet,
        params: CloseGameAccountParams,
        resp: ResponseHandle<CloseGameAccountResponse, CloseGameAccountError>
    ): Promise<void> {
        throw new Error('Method not implemented.')
    }

    async join(wallet: IWallet, params: JoinParams, resp: ResponseHandle<JoinResponse, JoinError>): Promise<void> {
        const { gameAddr, amount, position, verifyKey, createProfileIfNeeded = false } = params

        const suiClient = this.suiClient
        coerceWallet(wallet)

        if (createProfileIfNeeded) {
            let res = new ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>()
            await this.createPlayerProfile(wallet, { nick: wallet.walletAddr.substring(0, 6) }, res)
        }

        // get game object for token info and object ref
        const game = await this.getGameAccount(gameAddr)
        if (game == undefined) {
            console.error('Cannot join: game not found: ', gameAddr)
            return
        }

        if (game.players.length >= game.maxPlayers) {
            console.error('Cannot join: game already full: ', game.maxPlayers)
            return
        }
        // TODO: make this a procedure
        const objRes: SuiObjectResponse = await suiClient.getObject({
            id: gameAddr,
            options: { showOwner: true },
        })
        let found = false
        const owner: ObjectOwner | null = objRes.data?.owner ? objRes.data.owner : null
        const objectId = objRes.data?.objectId
        if (!owner || !objectId) {
            return resp.transactionFailed('get game object failed')
        }
        if (!(owner instanceof Object && 'Shared' in owner)) {
            return resp.transactionFailed('get game object onwer failed')
        }
        const shared = owner.Shared
        if (!('initial_shared_version' in shared)) {
            return resp.transactionFailed('game object is not shared')
        }
        const game_init_version = shared.initial_shared_version

        const transaction = new Transaction()

        // split coin for buyin
        const [coin] = transaction.splitCoins(transaction.gas, [transaction.pure.u64(params.amount)])
        // join the game
        transaction.moveCall({
            target: `${PACKAGE_ID}::game::join_game`,
            arguments: [
                transaction.sharedObjectRef({
                    objectId: gameAddr,
                    initialSharedVersion: game_init_version,
                    mutable: false,
                }),
                transaction.pure.u16(position),
                transaction.pure.u64(amount),
                transaction.pure.u64(verifyKey),
                coin,
            ],
            typeArguments: [game.tokenAddr],
        })

        const result = await wallet.send(transaction, suiClient, resp)
        if ('err' in result) {
            return resp.transactionFailed(result.err)
        }

        console.log(result)
    }

    async deposit(
        wallet: IWallet,
        params: DepositParams,
        resp: ResponseHandle<DepositResponse, DepositError>
    ): Promise<void> {
        const transaction = new Transaction()
        const suiClient = this.suiClient
        coerceWallet(wallet)

        throw new Error('Method not implemented.')
    }

    async createRecipient(
        wallet: IWallet,
        params: CreateRecipientParams,
        resp: ResponseHandle<CreateRecipientResponse, CreateRecipientError>
    ): Promise<void> {
        const transaction = new Transaction()
        const suiClient = this.suiClient
        coerceWallet(wallet)
        // 1. make move call to `new_recipient_builder` to get a hot potato
        let builder = transaction.moveCall({
            target: `${PACKAGE_ID}::recipient::new_recipient_builder`,
        })
        // 2. make a series of move calls to build recipient slots one by one
        let used_ids: number[] = []
        params.slots.forEach((slot: RecipientSlotInit) => {
            // slot id must be unique
            if (used_ids.includes(slot.id)) {
                return resp.transactionFailed('slot id must be unique')
            }
            used_ids.push(slot.id)
            // 2.1. create shares for this slot and collect them into a vector
            let result_shares: TransactionObjectArgument[] = []
            slot.initShares.forEach((share: RecipientSlotShareInit) => {
                if (share.owner === undefined) {
                    return resp.transactionFailed('share owner must be defined')
                }
                let owner_type
                let owner_info
                if ('addr' in share.owner) {
                    owner_type = 1
                    owner_info = share.owner.addr
                } else {
                    owner_type = 0
                    owner_info = share.owner.identifier
                }
                let result: TransactionObjectArgument = transaction.moveCall({
                    target: `${PACKAGE_ID}::recipient::create_slot_share`,
                    arguments: [
                        transaction.pure.u8(owner_type),
                        transaction.pure.string(owner_info),
                        transaction.pure.u16(share.weights),
                    ],
                })
                result_shares.push(result)
            })
            let shares = transaction.makeMoveVec({
                type: `${PACKAGE_ID}::recipient::RecipientSlotShare`,
                elements: result_shares,
            })
            let type_args
            if (slot.slotType === 'nft') {
                type_args = 0
            } else {
                type_args = 1
            }
            builder = transaction.moveCall({
                target: `${PACKAGE_ID}::recipient::create_recipient_slot`,
                arguments: [
                    transaction.pure.u8(slot.id),
                    transaction.pure.string(slot.tokenAddr),
                    transaction.pure.u8(type_args),
                    shares,
                    builder,
                ],
                typeArguments: [slot.tokenAddr],
            })
        })
        transaction.moveCall({
            target: `${PACKAGE_ID}::recipient::create_recipient`,
            arguments: [transaction.pure.option('address', wallet.walletAddr), builder],
        })
        const result = await wallet.send(transaction, suiClient, resp)
        if ('err' in result) {
            return resp.transactionFailed(result.err)
        }
    }

    async registerGame(
        wallet: IWallet,
        params: RegisterGameParams,
        resp: ResponseHandle<RegisterGameResponse, RegisterGameError>
    ): Promise<void> {
        coerceWallet(wallet)
        const transaction = new Transaction()
        const suiClient = this.suiClient
        const objectsRes: SuiObjectResponse[] = await suiClient.multiGetObjects({
            ids: [params.gameAddr, params.regAddr, CLOCK_ID],
            options: {
                showOwner: true,
            },
        })
        let objVersions: Record<string, number | string> = {}
        let getObjVersionRes = false
        for (const v of objectsRes) {
            const owner: ObjectOwner | null = v.data?.owner ? v.data.owner : null
            if (!owner) {
                getObjVersionRes = true
                break
            }
            if (!(owner instanceof Object && 'Shared' in owner)) {
                getObjVersionRes = true
                break
            }
            const shared = owner.Shared
            if (!('initial_shared_version' in shared)) {
                getObjVersionRes = true
                break
            }
            const objectId = v.data?.objectId
            if (!objectId) {
                getObjVersionRes = true
                break
            }
            objVersions[objectId] = shared.initial_shared_version
        }
        if (getObjVersionRes) {
            return resp.transactionFailed('get initial_shared_version failed')
        }
        transaction.moveCall({
            target: `${PACKAGE_ID}::registry::register_game`,
            arguments: [
                transaction.sharedObjectRef({
                    initialSharedVersion: objVersions[params.gameAddr],
                    mutable: false,
                    objectId: params.gameAddr,
                }),
                transaction.sharedObjectRef({
                    initialSharedVersion: objVersions[params.regAddr],
                    mutable: true,
                    objectId: params.regAddr,
                }),
                transaction.sharedObjectRef({
                    initialSharedVersion: objVersions[CLOCK_ID],
                    mutable: false,
                    objectId: CLOCK_ID,
                }),
            ],
        })
        const result = await wallet.send(transaction, suiClient, resp)
        if ('err' in result) {
            return resp.transactionFailed(result.err)
        }
        return resp.succeed({
            gameAddr: params.gameAddr,
            regAddr: params.regAddr,
        })
    }
    // todo contract
    unregisterGame(wallet: IWallet, params: UnregisterGameParams, resp: ResponseHandle): Promise<void> {
        throw new Error('Method not implemented.')
    }
    async getGameAccount(addr: string): Promise<GameAccount | undefined> {
        const suiClient = this.suiClient
        const resp: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showBcs: true,
                showType: true,
            },
        })

        return parseObjectData(parseSingleObjectResponse(resp), GameAccountParser)
    }

    async listGameAccounts(addrs: string[]): Promise<GameAccount[]> {
        let ret = []
        for (const addr of addrs) {
            const gameAccount = await this.getGameAccount(addr)
            if (gameAccount !== undefined) {
                ret.push(gameAccount)
            }
        }
        return ret
    }

    async getGameBundle(addr: string): Promise<GameBundle | undefined> {
        const suiClient = this.suiClient;
        const resp: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showBcs: true,
                showType: true
            }
        })
        return parseObjectData(parseSingleObjectResponse(resp), GameBundleParser)
    }

    async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
        const suiClient = this.suiClient;
        const resp: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showBcs: true,
                showType: true
            }
        })
        return parseObjectData(parseSingleObjectResponse(resp), ServerParser)
    }

    async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
        const suiClient = this.suiClient
        const resp: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showBcs: true,
                showType: true,
            },
        })
        return parseObjectData(
            parseSingleObjectResponse(resp),
            RegistrationAccountParser
        )
    }

    async getRecipient(addr: string): Promise<RecipientAccount | undefined> {
        const suiClient = this.suiClient
        const resp: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showBcs: true,
                showType: true,
            },
        })
        return parseObjectData(
            parseSingleObjectResponse(resp),
            RecipientAccountParser
        )
    }

    async getTokenDecimals(addr: string): Promise<number | undefined> {
        return this.getToken(addr).then(token => token?.decimals)
    }
    async getToken(addr: string): Promise<Token | undefined> {
        const suiClient = this.suiClient
        const tokenMetadata = await suiClient.getCoinMetadata({ coinType: addr })
        if (!tokenMetadata) return undefined
        const token: Token = {
            addr: addr,
            icon: tokenMetadata.iconUrl || SUI_ICON_URL,
            name: tokenMetadata.name,
            symbol: tokenMetadata.symbol,
            decimals: tokenMetadata.decimals,
        }
        return token
    }

    async getNft(addr: string): Promise<Nft | undefined> {
        const suiClient = this.suiClient
        const objectResponse: SuiObjectResponse = await suiClient.getObject({
            id: addr,
            options: {
                showContent: true,
                showType: true,
            },
        })
        if (objectResponse.error) {
            console.error('Error fetching NFT:', objectResponse.error)
            return undefined
        }
        const content = objectResponse.data?.content
        if (!content || content.dataType !== 'moveObject') {
            return undefined
        }
        if (!content.fields) return undefined
        let fields = content.fields
        if (Array.isArray(fields)) {
            return undefined
        }
        if ('fields' in fields) {
            return undefined
        }
        if ('balance' in fields) return undefined
        if (fields['image_url'] || fields['img_url']) {
            return {
                addr: addr,
                image: fields?.image_url?.toString() || fields?.img_url?.toString() || '',
                name: fields.name?.toString() || '',
                symbol: fields.symbol?.toString() || fields.name?.toString() || '',
                collection: objectResponse?.data?.type || undefined,
                metadata: objectResponse,
            }
        }
        return undefined
    }

    async listTokens(tokenAddrs: string[]): Promise<Token[]> {
        const promises = tokenAddrs.map(async addr => {
            return await this.getToken(addr)
        })
        let tokens = (await Promise.all(promises)).filter((t): t is Token => t !== undefined)
        return tokens
    }

    async listTokenBalance(walletAddr: string, tokenAddrs: string[], storage?: IStorage): Promise<TokenBalance[]> {
        return (await this.suiClient.getAllBalances({owner: walletAddr})).map(b => ({
            addr: b.coinType,
            amount: BigInt(b.totalBalance)
        })).filter(t => tokenAddrs.includes(t.addr))
    }

    async listNfts(walletAddr: string): Promise<Nft[]> {
        const suiClient = this.suiClient
        const tokenMetadata = await suiClient.getOwnedObjects({ owner: walletAddr })
        if (!tokenMetadata) return []
        let ids = tokenMetadata.data.map(obj => obj?.data?.objectId).filter((id): id is string => id !== undefined)
        const objectResponses: SuiObjectResponse[] = await suiClient.multiGetObjects({
            ids,
            options: {
                showContent: true,
                showType: true,
            },
        })
        let nfts = objectResponses
            .map((obj: SuiObjectResponse) => {
                const content = obj.data?.content
                if (!content || content.dataType !== 'moveObject') {
                    return undefined
                }
                if (!content.fields) return undefined
                let fields = content.fields
                if (Array.isArray(fields)) {
                    return undefined
                }
                if ('fields' in fields) {
                    return undefined
                }
                if ('balance' in fields) return undefined
                if (fields['image_url'] || fields['img_url']) {
                    return {
                        addr: walletAddr,
                        image: fields?.image_url?.toString() || fields?.img_url?.toString() || '',
                        name: fields.name?.toString() || '',
                        symbol: fields.symbol?.toString() || fields.name?.toString() || '',
                        collection: obj?.data?.type || undefined,
                        metadata: obj,
                    }
                }
                return undefined
            })
            .filter((obj: Nft | undefined): obj is Nft => obj !== undefined)
        return nfts
    }
    recipientClaim(
        wallet: IWallet,
        params: RecipientClaimParams,
        resp: ResponseHandle<RecipientClaimResponse, RecipientClaimError>
    ): Promise<void> {
        throw new Error('Method not implemented.')
    }

    attachBonus(
        wallet: IWallet,
        params: AttachBonusParams,
        resp: ResponseHandle<AttachBonusResponse, AttachBonusError>
    ): Promise<void> {
        throw new Error('Method not implemented.')
    }
}

function resolveObjectCreatedByType<T, E>(
    result: TxResult,
    objectType: string,
    resp: ResponseHandle<T, E>
): SuiObjectChangeCreated | undefined {
    if (!('objectChanges' in result)) {
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

function parseMultiObjectResponse(resp: PaginatedObjectsResponse): SuiObjectData[] {
    if ('error' in resp) {
        console.error(resp.error)
        return []
    }
    const objData = resp.data.map(x => x.data).filter((x): x is SuiObjectData => !!x)
    return objData
}

function parseFirstObjectResponse(resp: PaginatedObjectsResponse): SuiObjectData | undefined {
    if ('error' in resp) {
        console.error(resp.error)
        return undefined
    }
    const objData = resp.data.at(0)?.data
    return objData ? objData : undefined
}

function parseSingleObjectResponse(resp: SuiObjectResponse): SuiObjectData | undefined {
    if ('error' in resp) {
        console.error(resp.error)
        return undefined
    }
    const objData = resp.data
    return objData ? objData : undefined
}

function parseObjectData<T, S extends BcsType<S['$inferInput']>>(
    objData: SuiObjectData | undefined,
    parser: Parser<T, S>,
): undefined | T {
    if (objData === undefined) {
        return undefined
    }

    const data = objData.bcs

    if (!data) {
        console.warn('BCS data is empty')
        return undefined
    }

    if (data.dataType === 'package') {
        console.error('Not a move object')
        return undefined
    }

    const raw = parser.schema.parse(fromBase64(data.bcsBytes))

    return parser.transform(raw)
}
