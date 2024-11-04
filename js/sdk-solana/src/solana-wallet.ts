import { Connection, TransactionInstruction, TransactionSignature } from '@solana/web3.js'
import { IWallet, SendTransactionResult } from '@race-foundation/sdk-core'

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

  get wallet(): any {
    return this.#wallet;
  }
}
