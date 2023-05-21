import { field, array, enums, option, variant, struct } from '@race/borsh';
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
  @field('usize')
  randomId!: number;
  @field('usize')
  index!: number;
  @field('u8-array')
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
  @field('usize')
  decisionId!: number;
  @field('u8-array')
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
  @field('u8-array')
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
  @field(array(enums(SecretShare)))
  shares!: SecretShare[];
  constructor(fields: Fields<ShareSecrets>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(3)
export class OperationTimeout extends GameEvent {
  @field(array('string'))
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
  @field('usize')
  randomId!: number;
  @field(array('u8-array'))
  ciphertexts!: Uint8Array[];
  constructor(fields: Fields<Mask>) {
    super();
    Object.assign(this, fields);
  }
}

export class CiphertextAndDigest {
  @field('u8-array')
  ciphertext!: Uint8Array;
  @field('u8-array')
  digest!: Uint8Array;
  constructor(fields: Fields<CiphertextAndDigest>) {
    Object.assign(this, fields);
  }
}

@variant(5)
export class Lock extends GameEvent {
  @field('string')
  sender!: string;
  @field('usize')
  randomId!: number;
  @field(array(struct(CiphertextAndDigest)))
  ciphertextsAndDigests!: CiphertextAndDigest[];
  constructor(fields: Fields<Lock>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(6)
export class RandomnessReady extends GameEvent {
  @field('usize')
  randomId!: number;
  constructor(fields: Fields<RandomnessReady>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(7)
export class Sync extends GameEvent {
  @field(array(struct(PlayerJoin)))
  newPlayers!: PlayerJoin[];
  @field(array(struct(ServerJoin)))
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
  constructor(_: any = {}) {
    super();
  }
}

@variant(12)
export class DrawRandomItems extends GameEvent {
  @field('string')
  sender!: string;
  @field('usize')
  randomId!: number;
  @field(array('usize'))
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
  @field('usize')
  decisionId!: number;
  @field('u8-array')
  ciphertext!: Uint8Array;
  @field('u8-array')
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
