import { Connection, Keypair, TransactionInstruction, TransactionMessage, VersionedTransaction } from '@solana/web3.js';
import { IWallet, TransactionResult } from '@race-foundation/sdk-core';

export class SolanaFileWalletAdapter implements IWallet {
  keypair: Keypair

  get walletAddr(): string {
    return this.keypair.publicKey.toBase58();
  }

  get isConnected(): boolean {
    return true;
  }

  constructor(keyfilePath: string) {
    const fs = require('fs');
    const v = JSON.parse(fs.readFileSync(keyfilePath));
    this.keypair = Keypair.fromSecretKey(Uint8Array.from(v));
  }

  async sendTransaction(tx: TransactionInstruction, conn: Connection): Promise<TransactionResult<void>> {
    const {
      context: { slot: _minContextSlot },
      value: { blockhash, lastValidBlockHeight },
    } = await conn.getLatestBlockhashAndContext();

    const message = new TransactionMessage({
      payerKey: this.keypair.publicKey,
      recentBlockhash: blockhash,
      instructions: [tx]
    }).compileToV0Message();
    const transaction = new VersionedTransaction(message);
    transaction.sign([this.keypair]);
    const signature = await conn.sendTransaction(transaction)
    const resp = await conn.confirmTransaction({ blockhash, lastValidBlockHeight, signature });
    if (resp.value.err !== null) {
      return {
        result: 'err', error: resp.value.err.toString()
      }
    } else {
      return {
        result: 'ok'
      }
    }
  }
}
