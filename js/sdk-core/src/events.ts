import * as borsh from "borsh";
import { ExtendedWriter, ExtendedReader } from './utils';
import { Buffer } from 'buffer';
import { PlayerJoin, ServerJoin } from './accounts';

export interface IRandom {
  fromAddr: string;
  toAddr: string | undefined;
  randomId: bigint;
  index: number;
  secret: Uint8Array;
}

export interface IAnswer {
  fromAddr: string;
  decisionId: bigint;
  secret: Uint8Array;
}

export enum EventKind {
  Custom = 0,
  Ready,
  ShareSecrets,
  SecretsReady,
  Shutdown,
}

export interface IGameEvent {
  kind: EventKind;
}

export interface ICustom {
  sender: string;
  raw: Uint8Array;
}

export enum ShareKind {
  Random = 0,
  Answer,
}

export interface ISecretShare {
  kind: ShareKind;
}

export interface IShareSecrets {
  sender: string;
  shares: SecretShare[];
}

export interface IReady { }
export interface ISecretsReady { }
export interface IShutdown { }

export class Random implements IRandom, ISecretShare {
  fromAddr!: string;
  toAddr!: string | undefined;
  randomId!: bigint;
  index!: number;
  secret!: Uint8Array;
  constructor(fields: IRandom) {
    Object.assign(this, fields);
  }
  get kind() { return ShareKind.Random }
  static get schema(): Map<Function, any> {
    return new Map([
      [Random, {
        kind: 'struct',
        fields: [
          ['kind', 'u8'],
          ['fromAddr', 'string'],
          ['toAddr', 'string'],
          ['randomId', 'bigint'],
          ['index', 'u32'],
          ['secret', 'bytes'],
        ]
      }]
    ])
  }
}

export class Answer implements IAnswer, ISecretShare {
  fromAddr!: string;
  decisionId!: bigint;
  secret!: Uint8Array;
  constructor(fields: IAnswer) {
    Object.assign(this, fields)
  }
  get kind() { return ShareKind.Answer }
  static get schema(): Map<Function, any> {
    return new Map([
      [Answer, {
        kind: 'struct',
        fields: [
          ['kind', 'u8'],
          ['fromAddr', 'string'],
          ['decisionId', 'bigint'],
          ['secret', 'bytes']
        ]
      }]
    ])
  }
}

export type SecretShare = Answer | Random;

export class Custom implements ICustom, IGameEvent {
  sender: string;
  raw: Uint8Array;
  constructor(fields: ICustom) {
    this.sender = fields.sender;
    this.raw = fields.raw;
  }
  get kind() { return EventKind.Custom }
  static get schema(): Map<Function, any> {
    return new Map([
      [Custom, {
        kind: 'struct',
        fields: [
          ['kind', 'u8'],
          ['sender', 'string'],
          ['raw', 'bytes']
        ]
      }]
    ])
  }
}

export class Ready implements IReady, IGameEvent {
  constructor(_: any = {}) { }
  get kind() { return EventKind.Ready }
  static get schema(): Map<Function, any> {
    return new Map([[Ready, { kind: 'struct', fields: [['kind', 'u8']] }]])
  }
  static default(): Ready {
    return new Ready({});
  }
}

export class ShareSecrets implements IShareSecrets, IGameEvent {
  sender: string;
  shares: SecretShare[];
  constructor(fields: IShareSecrets) {
    this.sender = fields.sender;
    this.shares = fields.shares;
  }
  get kind() { return EventKind.ShareSecrets };
  static get schema(): Map<Function, any> {
    return new Map([
      [ShareSecrets, {
        kind: 'struct',
        fields: [
          ['sender', 'string'],
          ['shares', []]
        ]
      }]
    ]);
  }
}


export class SecretsReady implements ISecretsReady, IGameEvent {
  constructor(_: any = {}) { }
  get kind() { return EventKind.SecretsReady }
  static get schema(): Map<Function, any> {
    return new Map([[SecretsReady, { kind: 'struct', fields: [['kind', 'u8']] }]])
  }
  static default(): SecretsReady {
    return new SecretsReady({});
  }
}

export class Shutdown implements IShutdown, IGameEvent {
  constructor(_: any = {}) { }
  get kind() { return EventKind.Shutdown }
  static get schema(): Map<Function, any> {
    return new Map([[Shutdown, { kind: 'struct', fields: [['kind', 'u8']] }]])
  }
  static default(): Shutdown {
    return new Shutdown({});
  }
}

export type EventValue = Custom
  | Ready
  | ShareSecrets
  | SecretsReady
  | Shutdown;

export class GameEvent {
  kind: EventKind;
  value: EventValue;
  constructor(kind: EventKind, value: EventValue) {
    this.kind = kind;
    this.value = value;
  }
  static deserialize(data: Uint8Array) {
    const kind = data[0] as EventKind;
    const buf = Buffer.from(data.slice(1));
    let value;
    switch (kind) {
      case EventKind.Custom:
        value = borsh.deserialize(Custom.schema, Custom, buf, ExtendedReader);
        break;
      case EventKind.Ready:
        value = Ready.default;
        break;
      case EventKind.ShareSecrets:
        value = borsh.deserialize(ShareSecrets.schema, , )
      case EventKind.SecretsReady:
      case EventKind.Shutdown:
    }
    return new GameEvent(kind, value);
  }
}

export function serializeEvent(event: GameEvent): Uint8Array {
  switch (event.kind) {
    case EventKind.Custom:
      return borsh.serialize(Custom.schema, event, ExtendedWriter);
    case EventKind.Ready:
      return borsh.serialize(Ready.schema, event, ExtendedWriter);
    case EventKind.ShareSecrets:
      return borsh.serialize(ShareSecrets.schema, event, ExtendedWriter);
    case EventKind.SecretsReady:
      return borsh.serialize(SecretsReady.schema, event, ExtendedWriter);
    case EventKind.Shutdown:
      return borsh.serialize(Shutdown.schema, event, ExtendedWriter);
  }
}

export function deserializeEvent(data: Uint8Array): GameEvent {
  let kind = data[0] as EventKind;
  switch (kind) {
    case EventKind.Custom:
      return borsh.deserialize(Custom.schema, Custom, Buffer.from(data), ExtendedReader);
    case EventKind.Ready:
      return borsh.deserialize(Ready.schema, Ready, Buffer.from(data), ExtendedReader);
  }
}

export type _GameEvent =
  {
    Custom: {
      sender: string;
      raw: string;
    }
  }
  | "Ready"
  | {
    ShareSecrets: {
      sender: string;
      shares: SecretShare[];
    }
  }
  | {
    OperationTimeout: {
      addrs: string[];
    }
  }
  | {
    Mask: {
      sender: string;
      random_id: bigint;
      ciphertexts: Uint8Array,
    }
  }
  | {
    Lock: {
      sender: string;
      random_id: bigint;
      ciphertexs_and_digests: Array<[Uint8Array, Uint8Array]>;
    }
  }
  | {
    RandomnessReady: {
      random_id: bigint,
    }
  }
  | {
    Sync: {
      new_players: PlayerJoin[],
      new_servers: ServerJoin[],
      transactor_addr: string;
      access_version: bigint,
    }
  }
  | {
    ServerLeave: {
      server_addr: string;
      transactor_addr: string;
    }
  }
  | {
    Leave: {
      player_addr: string;
    }
  }
  | {
    GameStart: {
      access_version: bigint;
    }
  }
  | "WaitingTimeout"
  | {
    DrawRandomItems: {
      sender: string;
      random_id: number;
      indexes: number[];
    }
  }
  | "DrawTimeout"
  | {
    ActionTimeout: {
      player_addr: string;
    }
  }
  | {
    AnswerDecision: {
      sender: string;
      decision_id: bigint;
      ciphertext: Uint8Array;
      digest: Uint8Array;
    }
  }
  | "SecretsReady"
  | "Shutdown";


export interface ICustomEvent {
  serialize(): Uint8Array;
  deserialize(data: Uint8Array): ICustomEvent;
}

export function makeCustomEvent(sender: string, customEvent: ICustomEvent): Custom {
  return new Custom({
    sender,
    raw: customEvent.serialize(),
  });
}
