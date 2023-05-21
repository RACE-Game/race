import { RandomSpec, RandomState } from './random-state';
import { HandlerError } from './error';
import { GameContext } from './game-context';
import { enums, field, map, option, struct, variant, array } from '@race/borsh';
import { Fields } from './types';

export abstract class SettleOp {}

@variant(0)
export class SettleAdd extends SettleOp {
  @field('u64')
  amount!: bigint;
  constructor(fields: Fields<SettleAdd>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(1)
export class SettleSub extends SettleOp {
  @field('u64')
  amount!: bigint;
  constructor(fields: Fields<SettleAdd>) {
    super();
    Object.assign(this, fields);
  }
}

@variant(2)
export class SettleEject extends SettleOp {
  constructor(_: any) {
    super();
  }
}

export class Settle {
  @field('string')
  addr: string;
  @field(enums(SettleOp))
  op: SettleOp;
  constructor(fields: { addr: string; op: SettleOp }) {
    this.addr = fields.addr;
    this.op = fields.op;
  }
  sortKey(): number {
    if (this.op instanceof SettleAdd) {
      return 0;
    } else if (this.op instanceof SettleSub) {
      return 1;
    } else {
      return 2;
    }
  }
  compare(s: Settle): number {
    return this.sortKey() - s.sortKey();
  }
}

export class Ask {
  @field('string')
  playerAddr!: string;
  constructor(fields: Fields<Ask>) {
    Object.assign(this, fields);
  }
}

export class Assign {
  @field('usize')
  randomId!: number;
  @field('string')
  playerAddr!: string;
  @field(array('usize'))
  indexes!: number[];
  constructor(fields: Fields<Assign>) {
    Object.assign(this, fields);
  }
}

export class Reveal {
  @field('usize')
  randomId!: number;
  @field(array('usize'))
  indexes!: number[];
  constructor(fields: Fields<Reveal>) {
    Object.assign(this, fields);
  }
}

export class Release {
  @field('usize')
  decisionId!: number;
  constructor(fields: Fields<Release>) {
    Object.assign(this, fields);
  }
}

export class ActionTimeout {
  @field('string')
  playerAddr!: string;
  @field('u64')
  timeout!: bigint;
  constructor(fields: Fields<ActionTimeout>) {
    Object.assign(this, fields);
  }
}

export class Effect {
  @field(option(struct(ActionTimeout)))
  actionTimeout: ActionTimeout | undefined;

  @field(option('u64'))
  waitTimeout: bigint | undefined;

  @field('bool')
  startGame!: boolean;

  @field('bool')
  stopGame!: boolean;

  @field('bool')
  cancelDispatch!: boolean;

  @field('u64')
  timestamp!: bigint;

  @field('usize')
  currRandomId!: number;

  @field('usize')
  currDecisionId!: number;

  @field('u16')
  playersCount!: number;

  @field('u16')
  serversCount!: number;

  @field(array(struct(Ask)))
  asks!: Ask[];

  @field(array(struct(Assign)))
  assigns!: Assign[];

  @field(array(struct(Reveal)))
  reveals!: Reveal[];

  @field(array(struct(Release)))
  releases!: Release[];

  @field(array(enums(RandomSpec)))
  initRandomStates!: RandomSpec[];

  @field(map('usize', map('usize', 'string')))
  revealed!: Map<number, Map<number, string>>;

  @field(map('usize', 'string'))
  answered!: Map<number, string>;

  @field(array(struct(Settle)))
  settles!: Settle[];

  @field(option('u8-array'))
  handlerState!: Uint8Array | undefined;

  @field(option(enums(HandlerError)))
  error: HandlerError | undefined;

  @field('bool')
  allowExit!: boolean;

  constructor(fields: Fields<Effect>) {
    Object.assign(this, fields);
  }

  static fromContext(context: GameContext) {
    const actionTimeout = undefined;
    const waitTimeout = undefined;
    const startGame = false;
    const stopGame = false;
    const cancelDispatch = false;
    const timestamp = context.timestamp;
    const currRandomId = context.randomStates.length + 1;
    const currDecisionId = context.decisionStates.length + 1;
    const playersCount = context.players.length;
    const serversCount = context.servers.length;
    const asks: Ask[] = [];
    const assigns: Assign[] = [];
    const releases: Release[] = [];
    const reveals: Reveal[] = [];
    const initRandomStates: RandomSpec[] = [];
    const revealed = new Map();
    const answered = new Map();
    const settles: Settle[] = [];
    const handlerState = context.handlerState;
    const error = undefined;
    const allowExit = context.allowExit;
    return new Effect({
      actionTimeout,
      waitTimeout,
      startGame,
      stopGame,
      cancelDispatch,
      timestamp,
      currRandomId,
      currDecisionId,
      playersCount,
      serversCount,
      asks,
      assigns,
      releases,
      reveals,
      initRandomStates,
      revealed,
      answered,
      settles,
      handlerState,
      error,
      allowExit,
    });
  }
}
