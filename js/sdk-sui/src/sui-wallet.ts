import { SuiClient } from '@mysten/sui/dist/cjs/client'
import { IWallet, ResponseHandle, Result } from '@race-foundation/sdk-core'
import { Transaction } from '@mysten/sui/transactions'
import { WalletAdapter } from '@suiet/wallet-sdk'
import { SuiSignAndExecuteTransactionOutput } from '@mysten/wallet-standard'
import type { IdentifierString, WalletAccount } from '@wallet-standard/core';
import { ISigner } from './signer'

export class SuiWallet implements IWallet, ISigner {

  wallet: any
  chain: IdentifierString

  constructor(chain: IdentifierString, wallet: WalletAdapter) {
    this.wallet = wallet
    this.chain = chain
  }

  async send<T, E>(transaction: Transaction, _: SuiClient, resp: ResponseHandle<T, E>): Promise<Result<SuiSignAndExecuteTransactionOutput, string>> {
    resp.waitingWallet()
    const result = await this.wallet.features['sui:signAndExecuteTransaction'].signAndExecuteTransaction({
      transaction, account: this.wallet.accounts[0], chain: this.chain
    })
    return { ok: result }
  }

  get isConnected(): boolean {
    return this.wallet.accounts.length > 0
  }

  get walletAddr(): string {
    return this.wallet.accounts[0].address
  }
}
