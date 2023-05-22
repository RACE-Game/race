import { nanoid } from 'nanoid';
import { IEncryptor, IPublicKeyRaws, PublicKeyRaws } from './encryptor';
import { GameEvent } from './events';
import { deserialize, enums, field, serialize, struct, variant } from '@race/borsh';
import { base64ToArrayBuffer, arrayBufferToBase64, base64ToUint8Array } from './utils';

type Method = 'attach_game' | 'submit_event' | 'exit_game' | 'subscribe_event';


interface IAttachGameParams {
  key: PublicKeyRaws;
  signer: string;
}

interface ISubscribeEventParams {
  settleVersion: bigint;
}

interface ISubmitEventParams {
  event: GameEvent
}


export class AttachGameParams implements IAttachGameParams {
  @field(struct(PublicKeyRaws))
  key: PublicKeyRaws;
  @field('string')
  signer: string;
  constructor(fields: IAttachGameParams) {
    this.key = fields.key;
    this.signer = fields.signer;
  }
}

export class ExitGameParams { }


export class SubscribeEventParams implements ISubscribeEventParams {
  @field('u64')
  settleVersion: bigint;
  constructor(fields: ISubscribeEventParams) {
    this.settleVersion = fields.settleVersion;
  }
}

export class SubmitEventParams implements ISubmitEventParams {
  @field(enums(GameEvent))
  event: GameEvent;
  constructor(fields: ISubmitEventParams) {
    this.event = fields.event;
  }
}

export abstract class BroadcastFrame { }

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

  subscribeEvents(gameAddr: string, params: SubscribeEventParams): AsyncGenerator<BroadcastFrame | undefined>;
}

function* EventStream() { }

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
    const req = this.makeReqNoSig(gameAddr, 'attach_game', params);
    await this.requestXhr(req);
  }

  async submitEvent(gameAddr: string, params: SubmitEventParams): Promise<void> {
    const req = await this.makeReq(gameAddr, 'submit_event', params);
    await this.requestXhr(req);
  }

  async exitGame(gameAddr: string, params: ExitGameParams): Promise<void> {
    const req = await this.makeReq(gameAddr, 'exit_game', params);
    await this.requestXhr(req);
  }

  async *subscribeEvents(gameAddr: string, params: SubscribeEventParams): AsyncGenerator<BroadcastFrame | undefined> {
    const req = this.makeReqNoSig(gameAddr, 'subscribe_event', params);
    await this.requestWs(req);
    let messageQueue: BroadcastFrame[] = [];
    let resolve: undefined | ((value: BroadcastFrame | undefined) => void);
    let messagePromise = new Promise<BroadcastFrame | undefined>(r => (resolve = r));

    this.#socket.onmessage = msg => {
      if (resolve !== undefined) {
        let frame = this.parseEventMessage(msg.data);
        if (frame !== undefined) {
          let r = resolve;
          resolve = undefined;
          r(frame);
        }
      } else {
        let frame = this.parseEventMessage(msg.data);
        if (frame !== undefined) {
          messageQueue.push(frame);
        }
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
        // await new Promise(resolve => setTimeout(() => resolve(undefined), 100));
        // const p = messagePromise;
        // messagePromise = new Promise<BroadcastFrame | undefined>(r => (resolve = r));
        // yield p;
        yield messagePromise;
        messagePromise = new Promise<BroadcastFrame | undefined>(r => (resolve = r));
      }
    }
  }

  parseEventMessage(raw: string): BroadcastFrame | undefined {
    let resp = JSON.parse(raw);
    if (resp.method === 's_event') {
      let result: string = resp.params.result;
      let data = base64ToUint8Array(result);
      let frame = deserialize(BroadcastFrame, data);
      return frame;
    } else {
      return undefined;
    }
  }

  static initialize(playerAddr: string, endpoint: string, encryptor: IEncryptor): Connection {
    return new Connection(playerAddr, endpoint, encryptor);
  }

  async makeReq<P>(gameAddr: string, method: Method, params: P): Promise<string> {
    const paramsBytes = serialize(params);
    const sig = await this.#encryptor.sign(paramsBytes, this.#playerAddr);
    const sigBytes = serialize(sig);
    return JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: nanoid(),
      params: [gameAddr, arrayBufferToBase64(paramsBytes), arrayBufferToBase64(sigBytes)]
    });
  }

  makeReqNoSig<P>(gameAddr: string, method: Method, params: P): string {
    const paramsBytes = serialize(params);
    return JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: nanoid(),
      params: [gameAddr, arrayBufferToBase64(paramsBytes)]
    });
  }

  async requestWs(req: string): Promise<void> {
    try {
      await this.waitSocketReady();
      this.#socket.send(req);
    } catch (err) {
      console.error("Failed to connect to current transactor: " + this.#endpoint);
      throw err;
    }
  }

  async requestXhr<P>(req: string): Promise<P> {
    try {
      const resp = await fetch(this.#endpoint.replace(/^ws/, 'http'),
        {
          method: 'POST',
          body: req,
          headers: {
            'Content-Type': 'application/json',
          },
        });
      if (resp.ok) {
        return resp.json();
      } else {
        throw Error('Transactor request failed:' + resp.json());
      }
    } catch (err) {
      console.error("Failed to connect to current transactor: " + this.#endpoint);
      throw err;
    }
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
