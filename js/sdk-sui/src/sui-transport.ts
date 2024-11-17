import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, INft, IStorage, IToken, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, TransactionResult, UnregisterGameParams, VoteParams } from "@race-foundation/sdk-core";
import { Chain } from './common'
import { Balance, getFullnodeUrl, SuiClient, SuiTransactionBlock } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions'
// import { getFaucetHost, requestSuiFromFaucetV0 } from '@mysten/sui/faucet';

// import { TransactionBlock } from '@mysten/sui.js';
export class SuiTransport implements ITransport {
    get chain(): Chain { return 'sui' }

    async createGameAccount(wallet?: IWallet, params?: CreateGameAccountParams): Promise<TransactionResult<string>> {
        // 引入所需的 Sui SDK 模块

        // 创建连接到 Sui 网络的 provider（这里以测试网为例）
        const suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });

        // 创建新的 Ed25519 密钥对
        // const keypair = Ed25519Keypair.generate();
        // const address = keypair.getPublicKey().toSuiAddress();
        // 0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e
        // suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2
        const keypair = Ed25519Keypair.fromSecretKey('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2')
        const address = keypair.getPublicKey().toSuiAddress()
        // const sig = await requestSuiFromFaucetV0({
        //     host: getFaucetHost('testnet'),
        //     recipient: address,
        // });

        console.log(`New address: ${address+'-'+keypair.getSecretKey()}`);

        // 创建交易块来调用合约中的创建游戏账户方法
        const createGameAccount = async () => {
            const transaction = new Transaction();

            // 假设游戏账户创建合约地址和函数名称
            const gameModuleAddress = '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192'; 
            const createGameAccountFunction = 'create_profile';

            // 在交易块中调用创建游戏账户的 Move 函数
            transaction.moveCall({
                target: `${gameModuleAddress}::profile::${createGameAccountFunction}`,
                arguments: [transaction.pure.string('yuumi'),transaction.pure.string(address)], // 地址作为参数传递
            });

            // 打印交易块的内容，方便调试
            console.log('transaction:',transaction);
            try {
                // 使用密钥对来签署交易
                // 发送交易并等待结果
                const result = await suiClient.signAndExecuteTransaction({
                    transaction: transaction,
                    signer: keypair,
                    requestType: 'WaitForLocalExecution',
                    options: {
                        showEffects: true,
                    },
                });

                // 输出交易结果
                console.log('Transaction Result:', result);
            } catch (error) {
                console.error('Error while creating game account:', error);
            }
        };
        // 执行创建游戏账户的操作
        createGameAccount();
        return {result:'ok', value: '12'}
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
    createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<TransactionResult<void>> {
        throw new Error("Method not implemented.");
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
    getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
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
const suiTransport = new SuiTransport()
suiTransport.createGameAccount().then(result => {
    console.log('创建游戏账号success:', result)
})
.catch(error => {
    console.error('创建游戏账号时发生错误:', error);
});