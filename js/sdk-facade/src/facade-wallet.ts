import { nanoid } from 'nanoid';
import { IWallet } from 'race-sdk-core';

export class FacadeWallet implements IWallet {
  #addr: string;

  constructor() {
    this.#addr = nanoid();
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
