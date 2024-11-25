import { SuiClient } from '@mysten/sui/dist/cjs/client';
import { Transaction } from '@mysten/sui/dist/cjs/transactions';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { IWallet, SendTransactionResult } from '@race-foundation/sdk-core'
import { ISignature } from '@race-foundation/sdk-core/lib/types/encryptor';
import { ISigner } from './sui-transport';

export class LocalSuiWallet implements IWallet, ISigner {

  keypair: Ed25519Keypair
  address: string

  constructor() {
    this.keypair = Ed25519Keypair.fromSecretKey('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2')
    this.address = this.keypair.getPublicKey().toSuiAddress()
  }
  send(tx: Transaction, client: SuiClient): Promise<string> {
    return client.signAndExecuteTransaction(
      { tx, signer: this.keypair }
    )
  }

  sendTransaction(tx: any, conn: any): Promise<void> {
    throw new Error('Method not implemented.')
  }
  wallet: any

  signAndExecuteTransaction(transaction: Transaction, conn: SuiClient): any {
    return conn.signAndExecuteTransaction(
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
