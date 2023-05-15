import { deserialize, serialize, field, vec, enums, option, variant, struct } from '@race/borsh';
import { PlayerJoin, ServerJoin } from './accounts';
import { Fields } from './types';

export interface ICustomEvent {
  serialize(): Uint8Array;
  deserialize(data: Uint8Array): ICustomEvent;
}

export abstract class SecretShare {}

@variant(0)
export class Random extends SecretShare {
  @field('string')
  fromAddr!: string;
  @field(option('string'))
  toAddr!: string | undefined;
  @field('u64')
  randomId!: bigint;
  @field('u32')
  index!: number;
  @field(vec('u8'))
  secret!: Uint8Array;
  constructor(fields: Fields<Random>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(1)
export class Answer extends SecretShare {
  @field('string')
  fromAddr!: string;
  @field('u64')
  decisionId!: bigint;
  @field(vec('u8'))
  secret!: Uint8Array;
  constructor(fields: Fields<Answer>) {
    super();
    Object.assign(this, fields);
  }
}

export abstract class GameEvent {}

@variant(0)
export class Custom extends GameEvent {
  @field('string')
  sender!: string;
  @field(vec('u8'))
  raw!: Uint8Array;
  constructor(fields: Fields<Custom>) {
    super();
    Object.assign(this, fields);
  }
}

export function makeCustomEvent(sender: string, customEvent: ICustomEvent): Custom {
  return new Custom({
    sender,
    raw: customEvent.serialize(),
  });
}

@variant(1)
export class Ready extends GameEvent {
  constructor(_: any = {}) {
    super();
  }
}

@variant(2)
export class ShareSecrets extends GameEvent {
  @field('string')
  sender!: string;
  @field(vec(enums(SecretShare)))
  shares!: SecretShare[];
  constructor(fields: Fields<ShareSecrets>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(3)
export class OperationTimeout extends GameEvent {
  @field(vec('string'))
  addrs!: string[];
  constructor(fields: Fields<OperationTimeout>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(4)
export class Mask extends GameEvent {
  @field('string')
  sender!: string;
  @field('u64')
  randomId!: bigint;
  @field(vec(vec('u8')))
  ciphertexts!: Uint8Array[];
  constructor(fields: Fields<Mask>) {
    super();
    Object.assign(this, fields);
  }
}

export class CiphertextAndDigest {
  @field(vec('u8'))
  ciphertext!: Uint8Array;
  @field(vec('u8'))
  digest!: Uint8Array;
  constructor(fields: Fields<CiphertextAndDigest>) {
    Object.assign(this, fields);
  }
}

@variant(5)
export class Lock extends GameEvent {
  @field('string')
  sender!: string;
  @field('u64')
  randomId!: bigint;
  @field(vec(struct(CiphertextAndDigest)))
  ciphertextsAndDigests!: CiphertextAndDigest[];
  constructor(fields: Fields<Lock>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(6)
export class RandomnessReady extends GameEvent {
  @field('u64')
  randomId!: bigint;
  constructor(fields: Fields<RandomnessReady>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(7)
export class Sync extends GameEvent {
  @field(vec(struct(PlayerJoin)))
  newPlayers!: PlayerJoin[];
  @field(vec(struct(ServerJoin)))
  newServers!: ServerJoin[];
  @field('string')
  transactorAddr!: string;
  @field('u64')
  accessVersion!: bigint;
  constructor(fields: Fields<Sync>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(8)
export class ServerLeave extends GameEvent {
  @field('string')
  serverAddr!: string;
  @field('string')
  transactorAddr!: string;
  constructor(fields: Fields<ServerLeave>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(9)
export class Leave extends GameEvent {
  @field('string')
  playerAddr!: string;
  constructor(fields: Fields<Leave>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(10)
export class GameStart extends GameEvent {
  @field('u64')
  accessVersion!: bigint;
  constructor(fields: Fields<GameStart>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(11)
export class WaitingTimeout extends GameEvent {
  constructor(_: {}) {
    super();
  }
}

@variant(12)
export class DrawRandomItems extends GameEvent {
  @field('string')
  sender!: string;
  @field('u64')
  randomId!: bigint;
  @field(vec('u32'))
  indexes!: number[];
  constructor(fields: Fields<DrawRandomItems>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(13)
export class DrawTimeout extends GameEvent {
  constructor(_: {}) {
    super();
  }
}

@variant(14)
export class ActionTimeout extends GameEvent {
  @field('string')
  playerAddr!: string;
  constructor(fields: Fields<ActionTimeout>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(15)
export class AnswerDecision extends GameEvent {
  @field('string')
  sender!: string;
  @field('u64')
  decisionId!: bigint;
  @field(vec('u8'))
  ciphertext!: Uint8Array;
  @field(vec('u8'))
  digest!: Uint8Array;
  constructor(fields: Fields<AnswerDecision>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(16)
export class SecretsReady extends GameEvent {
  constructor(_: any = {}) {
    super();
  }
}

@variant(17)
export class Shutdown extends GameEvent {
  constructor(_: any = {}) {
    super();
  }
}
