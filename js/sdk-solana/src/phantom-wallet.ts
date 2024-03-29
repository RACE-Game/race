import { Connection, TransactionInstruction } from '@solana/web3.js';
import { IWallet, TransactionResult } from '@race-foundation/sdk-core';

const getProvider = () => {
  if ('phantom' in window) {
    const provider = (window.phantom as any)?.solana;

    if (provider?.isPhantom) {
      console.log('Phantom is installed');
      return provider;
    }
  }

  window.open('https://phantom.app/', '_blank');
};

interface SendOptions {
  skipPreflight: boolean;
}

export class PhantomWalletAdapter implements IWallet {
  #provider: any;
  #skipPreflight: boolean;

  get walletAddr(): string {
    return this.#provider.publicKey.toString();
  }

  get isConnected(): boolean {
    return this.#provider.isConnected;
  }

  get provider(): any {
    return this.#provider;
  }

  constructor(opts: SendOptions) {
    this.#provider = getProvider();
    this.#skipPreflight = opts.skipPreflight;
  }

  async sendTransaction(tx: TransactionInstruction, conn: Connection): Promise<TransactionResult<void>> {
    const {
      value: { blockhash, lastValidBlockHeight },
    } = await conn.getLatestBlockhashAndContext();
    const signedTransaction = await this.#provider.signTransaction(tx);
    const signature = await conn.sendRawTransaction(signedTransaction.serialize(), {
      skipPreflight: this.#skipPreflight,
    });
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

  async connect(): Promise<void> {
    await this.#provider.connect();
  }
}
