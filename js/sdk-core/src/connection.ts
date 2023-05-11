import { Chain } from './common';
import { IEncryptor } from './encryptor';
import { GameEvent } from './events';

export interface AttachGameParams {
  addr: string;
  key: string;
}

export interface GetStateParams {
  addr: string;
}

export interface SubscribeEventParams {
  addr: string;
}

export interface SubmitEventParams {
  addr: string;
  event: GameEvent;
}

export interface BroadcastFrame {
  gameAddr: string;
  state: any;
  event: GameEvent;
}

export interface IConnection {
  attachGame(params: AttachGameParams): Promise<void>;

  submitEvent(params: SubmitEventParams): Promise<void>;
}

export class Connection implements IConnection {
  #playerAddr: string;
  #endpoint: string;
  #encryptor: IEncryptor;

  constructor(playerAddr: string, endpoint: string, encryptor: IEncryptor) {
    this.#playerAddr = playerAddr;
    this.#endpoint = endpoint;
    this.#encryptor = encryptor;
  }

  async attachGame(params: AttachGameParams): Promise<void> {
  }

  async submitEvent(params: SubmitEventParams): Promise<void> {
  }

  static initialize(playerAddr: string, endpoint: string, encryptor: IEncryptor): Connection {
    return new Connection(playerAddr, endpoint, encryptor);
  }
}
