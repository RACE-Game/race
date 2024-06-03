import { field, array, enums, option, variant, struct } from '@race-foundation/borsh';
import { Fields, Id } from './types';
import { GamePlayer } from './init-account';

type EventFields<T> = Omit<Fields<T>, 'kind'>;

export type EventKind =
  | 'Invalid' // an invalid value
  | 'Custom'
  | 'Ready'
  | 'ShareSecrets'
  | 'OperationTimeout'
  | 'Mask'
  | 'Lock'
  | 'RandomnessReady'
  | 'Join'
  | 'ServerLeave'
  | 'Leave'
  | 'GameStart'
  | 'WaitingTimeout'
  | 'DrawRandomItems'
  | 'DrawTimeout'
  | 'ActionTimeout'
  | 'AnswerDecision'
  | 'SecretsReady'
  | 'Shutdown'
  | 'Bridge'
  // Client-only events
  | 'Init'
  | 'Checkpoint'
  | 'EndOfHistory';

export interface ICustomEvent {
  serialize(): Uint8Array;
}

export interface IBridgeEvent {
  serialize(): Uint8Array;
}

interface IEventKind {
  kind(): EventKind;
}

export abstract class SecretShare {}

@variant(0)
export class Random extends SecretShare {
  @field('string')
  fromAddr!: string;
  @field(option('string'))
  toAddr!: string | undefined;
  @field('usize')
  randomId!: Id;
  @field('usize')
  index!: number;
  @field('u8-array')
  secret!: Uint8Array;
  constructor(fields: EventFields<Random>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Random.prototype)
  }
}

@variant(1)
export class Answer extends SecretShare {
  @field('string')
  fromAddr!: string;
  @field('usize')
  decisionId!: Id;
  @field('u8-array')
  secret!: Uint8Array;
  constructor(fields: EventFields<Answer>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Answer.prototype)
  }
}

export abstract class GameEvent implements IEventKind {
  kind(): EventKind {
    return 'Invalid';
  }
}

export class EventHistory {
  @field(enums(GameEvent))
  event!: GameEvent;
  @field('u64')
  timestamp!: bigint;
  @field('string')
  stateSha!: string;
  constructor(fields: EventFields<Answer>) {
    Object.assign(this, fields);
    Object.setPrototypeOf(this, EventHistory.prototype);
  }
}

@variant(0)
export class Custom extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field('u8-array')
  raw!: Uint8Array;
  constructor(fields: EventFields<Custom>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Custom.prototype)
  }
  kind(): EventKind {
    return 'Custom';
  }
}

export function makeCustomEvent(sender: bigint, customEvent: ICustomEvent): Custom {
  return new Custom({
    sender,
    raw: customEvent.serialize(),
  });
}

@variant(1)
export class Ready extends GameEvent implements IEventKind {
  constructor(_: any = {}) {
    super();
    Object.setPrototypeOf(this, Ready.prototype)
  }
  kind(): EventKind {
    return 'Ready';
  }
}

@variant(2)
export class ShareSecrets extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field(array(enums(SecretShare)))
  shares!: SecretShare[];
  constructor(fields: EventFields<ShareSecrets>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, ShareSecrets.prototype)
  }
  kind(): EventKind {
    return 'ShareSecrets';
  }
}

@variant(3)
export class OperationTimeout extends GameEvent implements IEventKind {
  @field(array('u64'))
  ids!: bigint[];
  constructor(fields: EventFields<OperationTimeout>) {
    super();
    Object.setPrototypeOf(this, OperationTimeout.prototype)
    Object.assign(this, fields);
  }
  kind(): EventKind {
    return 'OperationTimeout';
  }
}

@variant(4)
export class Mask extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field('usize')
  randomId!: Id;
  @field(array('u8-array'))
  ciphertexts!: Uint8Array[];
  constructor(fields: EventFields<Mask>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Mask.prototype)
  }
  kind(): EventKind {
    return 'Mask';
  }
}

export class CiphertextAndDigest {
  @field('u8-array')
  ciphertext!: Uint8Array;
  @field('u8-array')
  digest!: Uint8Array;
  constructor(fields: EventFields<CiphertextAndDigest>) {
    Object.assign(this, fields);
  }
}

@variant(5)
export class Lock extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field('usize')
  randomId!: Id;
  @field(array(struct(CiphertextAndDigest)))
  ciphertextsAndDigests!: CiphertextAndDigest[];
  constructor(fields: EventFields<Lock>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Lock.prototype)
  }
  kind(): EventKind {
    return 'Lock';
  }
}

@variant(6)
export class RandomnessReady extends GameEvent implements IEventKind {
  @field('usize')
  randomId!: Id;
  constructor(fields: EventFields<RandomnessReady>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, RandomnessReady.prototype)
  }
  kind(): EventKind {
    return 'RandomnessReady';
  }
}

@variant(7)
export class Join extends GameEvent implements IEventKind {
  @field(array(struct(GamePlayer)))
  players!: GamePlayer[];
  constructor(fields: EventFields<Join>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Join.prototype)
  }
  kind(): EventKind {
    return 'Join';
  }
}

@variant(8)
export class ServerLeave extends GameEvent implements IEventKind {
  @field('u64')
  serverId!: bigint;
  constructor(fields: EventFields<ServerLeave>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, ServerLeave.prototype)
  }
  kind(): EventKind {
    return 'ServerLeave';
  }
}

@variant(9)
export class Leave extends GameEvent implements IEventKind {
  @field('u64')
  playerId!: bigint;
  constructor(fields: EventFields<Leave>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Leave.prototype)
  }
  kind(): EventKind {
    return 'Leave';
  }
}

@variant(10)
export class GameStart extends GameEvent implements IEventKind {
  constructor(_: any = {}) {
    super();
    Object.setPrototypeOf(this, GameStart.prototype)
  }
  kind(): EventKind {
    return 'GameStart';
  }
}

@variant(11)
export class WaitingTimeout extends GameEvent implements IEventKind {
  constructor(_: any = {}) {
    super();
    Object.setPrototypeOf(this, WaitingTimeout.prototype)
  }
  kind(): EventKind {
    return 'WaitingTimeout';
  }
}

@variant(12)
export class DrawRandomItems extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field('usize')
  randomId!: Id;
  @field(array('usize'))
  indexes!: number[];
  constructor(fields: EventFields<DrawRandomItems>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, DrawRandomItems.prototype)
  }
  kind(): EventKind {
    return 'DrawRandomItems';
  }
}

@variant(13)
export class DrawTimeout extends GameEvent implements IEventKind {
  constructor(_: {}) {
    super();
    Object.setPrototypeOf(this, DrawTimeout.prototype)
  }
  kind(): EventKind {
    return 'DrawTimeout';
  }
}

@variant(14)
export class ActionTimeout extends GameEvent implements IEventKind {
  @field('u64')
  playerId!: bigint;
  constructor(fields: EventFields<ActionTimeout>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, ActionTimeout.prototype)
  }
  kind(): EventKind {
    return 'ActionTimeout';
  }
}

@variant(15)
export class AnswerDecision extends GameEvent implements IEventKind {
  @field('u64')
  sender!: bigint;
  @field('usize')
  decisionId!: Id;
  @field('u8-array')
  ciphertext!: Uint8Array;
  @field('u8-array')
  digest!: Uint8Array;
  constructor(fields: EventFields<AnswerDecision>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, AnswerDecision.prototype)
  }
  kind(): EventKind {
    return 'AnswerDecision';
  }
}

@variant(16)
export class SecretsReady extends GameEvent implements IEventKind {
  @field(array('usize'))
  randomIds!: number[];

  constructor(fields: EventFields<SecretsReady>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, SecretsReady.prototype)
  }
  kind(): EventKind {
    return 'SecretsReady';
  }
}

@variant(17)
export class Shutdown extends GameEvent implements IEventKind {
  constructor(_: any = {}) {
    super();
    Object.setPrototypeOf(this, Shutdown.prototype)
  }
  kind(): EventKind {
    return 'Shutdown';
  }
}

@variant(18)
export class Bridge extends GameEvent implements IEventKind {
  @field('usize')
  dest!: number;
  @field('u8-array')
  raw!: Uint8Array;
  @field(array(struct(GamePlayer)))
  joinPlayers!: GamePlayer[];

  constructor(fields: EventFields<Bridge>) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, Bridge.prototype)
  }

  kind(): EventKind {
    return 'Bridge';
  }
}

// Client-only events, they can't be serialized and deserialized.

export class Init extends GameEvent implements IEventKind {
  constructor() {
    super();
  }
  kind(): EventKind {
    return 'Init'
  }
}

export class Checkpoint extends GameEvent implements IEventKind {
  constructor() {
    super();
  }
  kind(): EventKind {
    return 'Checkpoint'
  }
}

export class EndOfHistory extends GameEvent implements IEventKind {
  constructor() {
    super();
  }
  kind(): EventKind {
    return 'EndOfHistory'
  }
}
