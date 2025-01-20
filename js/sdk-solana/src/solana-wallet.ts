import { IWallet } from '@race-foundation/sdk-core'
import {
    address,
    getTransactionEncoder,
    SignatureBytes,
    Transaction,
    TransactionSendingSigner,
    TransactionSendingSignerConfig,
} from '@solana/web3.js'

export type SolanaChain = 'solana:mainnet' | 'solana:devnet' | 'solana:testnet' | 'solana:localhost'

export class SolanaWalletAdapter implements IWallet {
    #wallet: any
    #chain: SolanaChain

    constructor(chain: SolanaChain, wallet: any) {
        this.#wallet = wallet
        this.#chain = chain
    }

    get walletAddr(): string {
        return this.#wallet.accounts[0].address
    }

    get isConnected(): boolean {
        return this.#wallet.accounts.length > 0
    }

    get wallet(): TransactionSendingSigner {
        return {
            address: address(this.#wallet.accounts[0].address),
            signAndSendTransactions: async (
                transactions: readonly Transaction[],
                config?: TransactionSendingSignerConfig
            ): Promise<readonly SignatureBytes[]> => {
                if (transactions.length == 0) {
                    throw new Error('Transactions are empty')
                }
                if (transactions.length > 1) {
                    throw new Error('Cannot sign multiple transactions')
                }
                const transactionEncoder = getTransactionEncoder();
                const [transaction] = transactions;
                const wireTransactionBytes = transactionEncoder.encode(transaction);

                console.log(transactions, 'Sending transactions')
                console.log(config, "Transaction config")
                console.log('feature:', this.#wallet.features['solana:signAndSendTransaction'])
                const resps = await this.#wallet.features['solana:signAndSendTransaction'].signAndSendTransaction(
                    {
                        transaction: wireTransactionBytes,
                        chain: this.#chain,
                        account: this.#wallet.accounts[0],
                        options: {},
                    }
                )
                return resps.map((resp: any) => resp.signature)
            },
        }
    }
}
