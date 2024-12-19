import { SuiClient } from '@mysten/sui/dist/cjs/client'
import { IWallet, ResponseHandle, Result } from '@race-foundation/sdk-core'
import { Transaction } from '@mysten/sui/transactions'
import { WalletAdapter } from '@suiet/wallet-sdk'
import { SuiSignAndExecuteTransactionOutput } from '@mysten/wallet-standard'
import type { IdentifierString, WalletAccount } from '@wallet-standard/core';
import { ISigner } from './signer'

export class SuiWallet implements IWallet, ISigner {

  wallet: WalletAdapter
  account: WalletAccount
  chain: IdentifierString

  constructor(wallet: WalletAdapter, account: WalletAccount, chain: IdentifierString) {
    this.wallet = wallet
    this.account = account
    this.chain = chain
  }

  async send<T, E>(transaction: Transaction, _: SuiClient, resp: ResponseHandle<T, E>): Promise<Result<SuiSignAndExecuteTransactionOutput, string>> {
    resp.waitingWallet()
    const result = await this.wallet.signAndExecuteTransaction({
      transaction, account: this.account, chain: this.chain
    })
    return { ok: result }
  }

  get isConnected(): boolean {
    return true
  }

  get walletAddr(): string {
    return ''
  }
}
