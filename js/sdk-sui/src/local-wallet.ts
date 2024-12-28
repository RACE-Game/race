import { SuiClient, SuiTransactionBlockResponse } from '@mysten/sui/dist/cjs/client';
import { Transaction } from '@mysten/sui/dist/cjs/transactions';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { IWallet, ResponseHandle, Result } from '@race-foundation/sdk-core'
import { ISigner } from './signer';
import { GAS_BUDGET } from './constants';

export class LocalSuiWallet implements IWallet, ISigner {

  keypair: Ed25519Keypair
  address: string
  wallet: any

  constructor(privateKey: string) {
    this.keypair = Ed25519Keypair.fromSecretKey(privateKey)
    this.address = this.keypair.getPublicKey().toSuiAddress()
    console.log('this.address', this.address)
  }

  async send<T, E>(transaction: Transaction, client: SuiClient, resp: ResponseHandle<T, E>): Promise<Result<SuiTransactionBlockResponse, string>> {
    resp.waitingWallet()
    try {
      transaction.setGasBudget(GAS_BUDGET);
      const result = await client.signAndExecuteTransaction(
        { transaction, signer: this.keypair }
      )
      const digest = result.digest
      console.log('digest', digest)
      const blockResp = await client.getTransactionBlock({ digest, options: { showObjectChanges: true }})
      console.log('blockResp', blockResp)
      return { ok: blockResp }
    } catch (e: any) {
      resp.transactionFailed('Transaction failed')
      return { err: e.toString() }
    }
  }

  get isConnected(): boolean {
    return true
  }

  get walletAddr(): string {
    return this.address
  }
}
