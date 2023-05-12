import { Connection, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { IWallet } from '@race/sdk-core';

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

  async sendTransaction(tx: TransactionInstruction, conn: Connection): Promise<void> {
    const {
      context: { slot: minContextSlot },
      value: { blockhash, lastValidBlockHeight },
    } = await conn.getLatestBlockhashAndContext();
    const signature = await this.#wallet.sendTransaction(tx, conn, { minContextSlot });
    await conn.confirmTransaction({ blockhash, lastValidBlockHeight, signature });
  }
}
