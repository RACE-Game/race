import { Connection, TransactionInstruction, TransactionSignature } from '@solana/web3.js';
import { IWallet, TransactionResult } from '@race-foundation/sdk-core';

export class SolanaWalletAdapter implements IWallet {
  #wallet: any;

  get walletAddr(): string {
    return this.#wallet.publicKey.toBase58();
  }

  get isConnected(): boolean {
    return this.#wallet.connected;
  }

  constructor(wallet: any) {
    this.#wallet = wallet;
  }

  async sendTransaction(tx: TransactionInstruction, conn: Connection): Promise<TransactionResult<void>> {
    const {
      context: { slot: minContextSlot },
      value: { blockhash, lastValidBlockHeight },
    } = await conn.getLatestBlockhashAndContext();
    const signature: TransactionSignature = await this.#wallet.sendTransaction(tx, conn, { minContextSlot });
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
