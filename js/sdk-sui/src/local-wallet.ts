import { SuiClient } from '@mysten/sui/dist/cjs/client';
import { Transaction } from '@mysten/sui/dist/cjs/transactions';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { IWallet, TransactionResult } from '@race-foundation/sdk-core'

export class LocalSuiWallet implements IWallet {

  keypair: Ed25519Keypair
  address: string

  constructor() {
    this.keypair = Ed25519Keypair.fromSecretKey('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2')
    this.address = this.keypair.getPublicKey().toSuiAddress()
  }

  sendTransaction(tx: any, conn: any): Promise<TransactionResult<void>> {
    throw new Error('Method not implemented.')
  }

  signAndExecuteTransaction(transaction: Transaction, conn: SuiClient): any {
    conn.signAndExecuteTransaction(
      { transaction, signer: this.keypair }
    )
  }

  get isConnected(): boolean {
    return true
  }

  get walletAddr(): string {
    return this.address
  }
}
