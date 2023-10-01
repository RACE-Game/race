import { IEncryptor, PublicKeyRaws } from './encryptor';
import { TxState } from './tx-state';
import { GameEvent } from './events';
import { deserialize, array, enums, field, option, serialize, struct, variant } from '@race-foundation/borsh';
import { arrayBufferToBase64, base64ToUint8Array } from './utils';

export type ConnectionState = 'disconnected' | 'connected' | 'reconnected' | 'closed';

type Method = 'attach_game' | 'submit_event' | 'exit_game' | 'subscribe_event' | 'submit_message' | 'ping';

interface IAttachGameParams {
  signer: string;
  key: PublicKeyRaws;
}

interface ISubscribeEventParams {
  settleVersion: bigint;
}

interface ISubmitEventParams {
  event: GameEvent;
}

interface ISubmitMessageParams {
  content: string;
}

interface ICheckTxStateParams {
  newPlayers: string[];
  accessVersion: bigint;
}

export class AttachGameParams {
  @field('string')
  signer: string;
  @field(struct(PublicKeyRaws))
  key: PublicKeyRaws;

  constructor(fields: IAttachGameParams) {
    this.key = fields.key;
    this.signer = fields.signer;
  }
}

export class ExitGameParams {
  keepConnection?: boolean
}

export class SubscribeEventParams {
  @field('u64')
  settleVersion: bigint;
  constructor(fields: ISubscribeEventParams) {
    this.settleVersion = fields.settleVersion;
  }
}

export class SubmitEventParams {
  @field(enums(GameEvent))
  event: GameEvent;
  constructor(fields: ISubmitEventParams) {
    this.event = fields.event;
  }
}

export class SubmitMessageParams {
  @field('string')
  content: string;
  constructor(fields: ISubmitMessageParams) {
    this.content = fields.content;
  }
}

export class Message {
  @field('string')
  sender!: string;
  @field('string')
  content!: string;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
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
  @field('u8-array')
  checkpointState!: Uint8Array;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(2)
export class BroadcastFrameMessage extends BroadcastFrame {
  @field('string')
  gameAddr!: string;
  @field(struct(Message))
  message!: Message;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(3)
export class BroadcastFrameTxState extends BroadcastFrame {
  @field(enums(TxState))
  txState!: TxState;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

export interface IConnection {
  attachGame(params: AttachGameParams): Promise<void>;

  submitEvent(params: SubmitEventParams): Promise<ConnectionState | undefined>;

  submitMessage(params: SubmitMessageParams): Promise<ConnectionState | undefined>;

  exitGame(params: ExitGameParams): Promise<void>;

  connect(params: SubscribeEventParams): Promise<void>;

  subscribeEvents(): AsyncGenerator<BroadcastFrame | ConnectionState | undefined>;
}

export class Connection implements IConnection {
  gameAddr: string;
  playerAddr: string;
  endpoint: string;
  encryptor: IEncryptor;
  socket?: WebSocket;
  // If the connection is closed
  closed: boolean;

  // For async message stream
  streamResolve?: ((value: BroadcastFrame | ConnectionState | undefined) => void);
  streamMessageQueue: BroadcastFrame[];
  streamMessagePromise?: Promise<BroadcastFrame | ConnectionState | undefined>;

  // For keep alive
  lastPong: number;
  pingTimer?: any;
  checkTimer?: any;

  isFirstOpen: boolean;

  constructor(gameAddr: string, playerAddr: string, endpoint: string, encryptor: IEncryptor) {
    this.gameAddr = gameAddr;
    this.playerAddr = playerAddr;
    this.endpoint = endpoint;
    this.encryptor = encryptor;
    this.socket = undefined;
    this.closed = false;
    this.streamResolve = undefined;
    this.streamMessageQueue = [];
    this.streamMessagePromise = undefined;
    this.lastPong = new Date().getTime();
    this.isFirstOpen = true;
    this.pingTimer = undefined;
  }

  onDisconnected() {
    console.warn('Connection encountered an error, clean up connection');

    if (this.streamMessageQueue.find(x => x === 'disconnected') === undefined) {
      if (this.streamResolve !== undefined) {
        let r = this.streamResolve
        this.streamResolve = undefined;
        r('disconnected');
      } else {
        this.streamMessageQueue.push('disconnected');
      }
    }

    if (this.socket !== undefined) {
      this.socket.close();
    }
    this.socket = undefined;

    if (this.pingTimer !== undefined) {
      clearInterval(this.pingTimer);
      this.pingTimer = undefined;
    }

    if (this.checkTimer !== undefined) {
      clearInterval(this.checkTimer);
      this.checkTimer = undefined;
    }
  }

  async connect(params: SubscribeEventParams) {
    console.log('Establish server connection, settle version:', params.settleVersion);
    this.socket = new WebSocket(this.endpoint);

    if (this.checkTimer) {
      clearInterval(this.checkTimer);
      this.checkTimer = undefined;
    }

    this.socket.onmessage = msg => {
      const frame = this.parseEventMessage(msg.data);
      if (frame !== undefined) {
        if (this.streamResolve !== undefined) {
          let r = this.streamResolve;
          this.streamResolve = undefined;
          r(frame);
        } else {
          this.streamMessageQueue.push(frame);
        }
      }
    };

    this.socket.onopen = () => {
      let frame: ConnectionState;
      if (this.isFirstOpen) {
        frame = 'connected'
        this.isFirstOpen = false;
      } else {
        frame = 'reconnected'
      }

      // Start times for alive checking
      this.lastPong = new Date().getTime();
      this.pingTimer = setInterval(() => {
        if (this.socket !== undefined && this.socket.readyState === this.socket.OPEN) {
          this.socket.send(this.makeReqNoSig(this.gameAddr, 'ping', {}));
        }
      }, 3000);
      this.checkTimer = setInterval(() => {
        const t = new Date().getTime();
        if (this.lastPong + 6000 < t) {
          console.log("Websocket keep alive check failed, no reply for %s ms", t - this.lastPong);
          this.onDisconnected();
        }
      }, 500);

      if (this.streamResolve !== undefined) {
        let r = this.streamResolve;
        this.streamResolve = undefined;
        r(frame);
      } else {
        this.streamMessageQueue.push(frame);
      }
    }

    this.socket.onclose = () => {
      this.closed = true;
    }

    this.socket.onerror = (e) => {
      console.error(e);
      this.onDisconnected()
    };

    // Call JSONRPC subscribe_event
    const req = this.makeReqNoSig(this.gameAddr, 'subscribe_event', params);
    await this.requestWs(req);
  }

  async attachGame(params: AttachGameParams): Promise<void> {
    const req = this.makeReqNoSig(this.gameAddr, 'attach_game', params);
    await this.requestXhr(req);
  }

  async submitEvent(params: SubmitEventParams): Promise<ConnectionState | undefined> {
    try {
      const req = await this.makeReq(this.gameAddr, 'submit_event', params);
      await this.requestXhr(req);
      return undefined;
    } catch (_: any) {
      return 'disconnected';
    }
  }

  async submitMessage(params: SubmitMessageParams): Promise<ConnectionState | undefined> {
    try {
      const req = await this.makeReq(this.gameAddr, 'submit_message', params);
      await this.requestXhr(req);
      return undefined;
    } catch (_: any) {
      return 'disconnected';
    }
  }

  async exitGame(params: ExitGameParams): Promise<void> {
    const req = await this.makeReq(this.gameAddr, 'exit_game', {});
    await this.requestXhr(req);
    if (!params.keepConnection) {
      if (this.socket !== undefined) {
        this.closed = true;
        this.socket.close();
        this.socket = undefined;
      }
    }
  }

  async *subscribeEvents(): AsyncGenerator<BroadcastFrame | ConnectionState | undefined> {
    await this.waitSocketReady();
    this.streamMessagePromise = new Promise(r => (this.streamResolve = r));
    while (true) {
      if (this.streamMessageQueue.length > 0) {
        yield this.streamMessageQueue.shift()!;
      } else {
        yield this.streamMessagePromise;
        this.streamMessagePromise = new Promise(r => (this.streamResolve = r));
      }
    }
  }

  parseEventMessage(raw: string): BroadcastFrame | ConnectionState | undefined {
    let resp = JSON.parse(raw);
    if (resp.result === 'pong') {
      this.lastPong = new Date().getTime();
      return undefined;
    } else if (resp.method === 's_event') {
      if (resp.params.error === undefined) {
        let result: string = resp.params.result;
        let data = base64ToUint8Array(result);
        let frame = deserialize(BroadcastFrame, data);
        return frame;
      } else {
        return 'disconnected'
      }
    } else {
      return undefined;
    }
  }

  static initialize(gameAddr: string, playerAddr: string, endpoint: string, encryptor: IEncryptor): Connection {
    return new Connection(gameAddr, playerAddr, endpoint, encryptor);
  }

  async makeReq<P>(gameAddr: string, method: Method, params: P): Promise<string> {
    const paramsBytes = serialize(params);
    const sig = await this.encryptor.sign(paramsBytes, this.playerAddr);
    const sigBytes = serialize(sig);
    return JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: crypto.randomUUID(),
      params: [gameAddr, arrayBufferToBase64(paramsBytes), arrayBufferToBase64(sigBytes)],
    });
  }

  makeReqNoSig<P>(gameAddr: string, method: Method, params: P): string {
    const paramsBytes = serialize(params);
    return JSON.stringify({
      jsonrpc: '2.0',
      method,
      id: crypto.randomUUID(),
      params: [gameAddr, arrayBufferToBase64(paramsBytes)],
    });
  }

  async requestWs(req: string): Promise<void> {
    try {
      await this.waitSocketReady();
      if (this.socket !== undefined) {
        this.socket.send(req);
      }
    } catch (err) {
      console.error('Failed to connect to current transactor: ' + this.endpoint);
      throw err;
    }
  }

  async requestXhr<P>(req: string): Promise<P> {
    try {
      const resp = await fetch(this.endpoint.replace(/^ws/, 'http'), {
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
      console.error('Failed to connect to current transactor: ' + this.endpoint);
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
        } else if (this.socket !== undefined && this.socket.readyState === this.socket.OPEN) {
          clearInterval(interval);
          resolve(undefined);
        }
        currAttempt++;
      }, intervalTime);
    });
  }
}
