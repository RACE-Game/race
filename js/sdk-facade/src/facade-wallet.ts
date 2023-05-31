import { IWallet } from '@race-foundation/sdk-core';

function makeid(length: number) {
  let result = '';
  const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  const charactersLength = characters.length;
  let counter = 0;
  while (counter < length) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
    counter += 1;
  }
  return result;
}

export class FacadeWallet implements IWallet {
  #addr: string;

  constructor() {
    this.#addr = makeid(16);
  }

  get isConnected() {
    return true;
  }

  get walletAddr() {
    return this.#addr;
  }

  sendTransaction(_tx: any, _conn: any): Promise<void> {
    throw new Error('Method not implemented.');
  }
}
