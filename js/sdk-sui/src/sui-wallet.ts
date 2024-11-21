import { SuiClient } from '@mysten/sui/dist/cjs/client'
import { IWallet, TransactionResult } from '@race-foundation/sdk-core'
import { WalletContextState } from '@suiet/wallet-kit'
import { Transaction } from '@mysten/sui/transactions'
import { IWalletAdapter } from '@suiet/wallet-sdk'

export class SuiWallet implements IWallet {

  wallet: WalletContextState

  constructor(wallet: WalletContextState) {
    this.wallet = wallet
  }

  sendTransaction(tx: any, conn: any): Promise<TransactionResult<void>> {
    throw new Error('Method not implemented.')
  }

  signAndExecuteTransaction(transaction: Transaction, conn: SuiClient): any {
    this.wallet.signAndExecuteTransaction({ transaction })
  }

  get isConnected(): boolean {
    return true
  }

  get walletAddr(): string {
    return ''
  }
}
