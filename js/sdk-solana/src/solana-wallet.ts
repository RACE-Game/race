import { IWallet } from '@race-foundation/sdk-core'
import { SignerWalletAdapter } from '@solana/wallet-adapter-base'

export class SolanaWalletAdapter implements IWallet {
    #wallet: any

    get walletAddr(): string {
        return this.#wallet.publicKey.toBase58()
    }

    get isConnected(): boolean {
        return this.#wallet.connected
    }

    constructor(wallet: any) {
        this.#wallet = wallet
    }

    get wallet(): SignerWalletAdapter {
        return this.#wallet
    }
}
