import { nanoid } from 'nanoid';
import { IEncryptor } from './encryptor';
import { GameEvent } from './events';
import { enums, field, variant } from '@race/borsh';

export interface AttachGameParams {
  key: string;
}

export interface GetStateParams {
  addr: string;
}

export interface ExitGameParams {}

export interface SubscribeEventParams {
  settleVersion: bigint;
}

export interface SubmitEventParams {
  event: GameEvent;
}

export abstract class BroadcastFrame {}

@variant(0)
export class BroadcastFrameEvent extends BroadcastFrame {
  @field('string')
  gameAddr!: string;
  @field(enums(GameEvent))
  event!: GameEvent;
  @field('u64')
  timestamp!: bigint;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(1)
export class BroadcastFrameInit extends BroadcastFrame {
  @field('string')
  gameAddr!: string;
  @field('u64')
  accessVersion!: bigint;
  @field('u64')
  settleVersion!: bigint;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

export interface BroadcastFrame {
  gameAddr: string;
  state: any;
  event: GameEvent;
}

export interface IConnection {
  attachGame(gameAddr: string, params: AttachGameParams): Promise<void>;

  submitEvent(gameAddr: string, params: SubmitEventParams): Promise<void>;

  exitGame(gameAddr: string, params: ExitGameParams): Promise<void>;

  substribeEvents(gameAddr: string, params: SubscribeEventParams): AsyncGenerator<string | undefined>;
}

function* EventStream() {}

export class Connection implements IConnection {
  #playerAddr: string;
  #endpoint: string;
  #encryptor: IEncryptor;
  #socket: WebSocket;

  constructor(playerAddr: string, endpoint: string, encryptor: IEncryptor) {
    this.#playerAddr = playerAddr;
    this.#endpoint = endpoint;
    this.#encryptor = encryptor;
    const socket = new WebSocket(endpoint);
    this.#socket = socket;
  }

  async attachGame(gameAddr: string, params: AttachGameParams): Promise<void> {
    await this.request('attach_game', gameAddr, params);
  }

  async submitEvent(gameAddr: string, params: SubmitEventParams): Promise<void> {
    await this.request('submit_event', gameAddr, params);
  }

  async exitGame(gameAddr: string, params: ExitGameParams): Promise<void> {
    await this.request('exit_game', gameAddr, params);
  }

  async *substribeEvents(gameAddr: string, params: SubscribeEventParams): AsyncGenerator<string | undefined> {
    await this.request('subscribe_event', gameAddr, params);
    let messageQueue: string[] = [];
    let resolve: undefined | ((value: string | undefined) => void);
    let messagePromise = new Promise<string | undefined>(r => (resolve = r));

    this.#socket.onmessage = msg => {
      if (resolve !== undefined) {
        let r = resolve;
        resolve = undefined;
        r(msg.data);
      } else {
        messageQueue.push(msg.data);
      }
    };

    this.#socket.onclose = () => {
      if (resolve !== undefined) {
        let r = resolve;
        resolve = undefined;
        r(undefined);
      }
    };

    while (true) {
      if (messageQueue.length > 0) {
        yield messageQueue.shift()!;
      } else {
        yield messagePromise;
      }
    }
  }

  static initialize(playerAddr: string, endpoint: string, encryptor: IEncryptor): Connection {
    return new Connection(playerAddr, endpoint, encryptor);
  }

  async request<P>(method: string, gameAddr: string, params: P): Promise<void> {
    const message = gameAddr + '';
    const textEncoder = new TextEncoder();
    const signature = await this.#encryptor.sign(textEncoder.encode(message));
    console.log('Signature:', signature);
    const reqData = JSON.stringify(
      {
        jsonrpc: '2.0',
        method,
        id: nanoid(),
        params: [gameAddr, params, signature],
      },
      (_key, value) => (typeof value === 'bigint' ? Number(value) : value)
    );
    console.log('Request data:', reqData);
    await this.waitSocketReady();
    this.#socket.send(reqData);
  }

  waitSocketReady() {
    return new Promise((resolve, reject) => {
      let maxAttempts = 10;
      let intervalTime = 200;
      let currAttempt = 0;
      const interval = setInterval(() => {
        if (currAttempt > maxAttempts) {
          clearInterval(interval);
          reject();
        } else if (this.#socket.readyState === this.#socket.OPEN) {
          clearInterval(interval);
          resolve(undefined);
        }
        currAttempt++;
      }, intervalTime);
    });
  }
}
