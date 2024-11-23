import { SuiClient } from '@mysten/sui/dist/cjs/client'
import { IWallet, TransactionResult } from '@race-foundation/sdk-core'
import { Transaction } from '@mysten/sui/transactions'
// import { WalletAdapter } from '@suiet/wallet-sdk'
// import type { IdentifierString, WalletAccount } from '@wallet-standard/core';

type WalletAdapter = any
type WalletAccount = any
type IdentifierString = any
export class SuiWallet implements IWallet {

  wallet: WalletAdapter
  account: WalletAccount
  chain: IdentifierString

  constructor(wallet: WalletAdapter, account: WalletAccount, chain: IdentifierString) {
    this.wallet = wallet
    this.account = account
    this.chain = chain
  }

  sendTransaction(tx: any, conn: any): Promise<TransactionResult<void>> {
    throw new Error('Method not implemented.')
  }

  signAndExecuteTransaction(transaction: Transaction, conn: SuiClient): any {
    this.wallet.signAndExecuteTransaction({ transaction, account: this.account, chain: this.chain})
  }

  get isConnected(): boolean {
    return true
  }

  get walletAddr(): string {
    return ''
  }
}
