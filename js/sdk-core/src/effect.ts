import { RandomSpec } from './random-state';
import { HandleError } from './error';
import { GameContext } from './game-context';
import { enums, field, map, option, struct, variant, array } from '@race-foundation/borsh';
import { Fields, Id } from './types';
import { GamePlayer, InitAccount } from './init-account';

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
  @field('u64')
  id: bigint;
  @field(enums(SettleOp))
  op: SettleOp;
  constructor(fields: { id: bigint; op: SettleOp }) {
    this.id = fields.id;
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

export class Transfer {
  @field('u8')
  slotId!: number;
  @field('u64')
  amount!: bigint;
  constructor(fields: Fields<Transfer>) {
    Object.assign(this, fields);
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
  @field('u64')
  playerId!: bigint;
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
  @field('u64')
  playerId!: bigint;
  @field('u64')
  timeout!: bigint;
  constructor(fields: Fields<ActionTimeout>) {
    Object.assign(this, fields);
  }
}

export class SubGame {
  @field('usize')
  gameId!: number;
  @field('string')
  bundleAddr!: string;
  @field(struct(InitAccount))
  initAccount!: InitAccount;
  @field('u8-array')
  checkpointState!: Uint8Array;
  constructor(fields: Fields<SubGame>) {
    Object.assign(this, fields)
  }
}

export class EmitBridgeEvent {
  @field('usize')
  dest!: number;
  @field('u8-array')
  raw!: Uint8Array;
  @field(array(struct(GamePlayer)))
  joinPlayers!: GamePlayer[];

  constructor(fields: Fields<EmitBridgeEvent>) {
    Object.assign(this, fields)
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
  nodesCount!: number;

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

  @field('bool')
  isCheckpoint!: boolean;

  @field(array(struct(Settle)))
  settles!: Settle[];

  @field(option('u8-array'))
  handlerState!: Uint8Array | undefined;

  @field(option(enums(HandleError)))
  error: HandleError | undefined;

  @field('bool')
  allowExit!: boolean;

  @field(array(struct(Transfer)))
  transfers!: Transfer[];

  @field(array(struct(SubGame)))
  launchSubGames!: SubGame[];

  @field(array(struct(EmitBridgeEvent)))
  bridgeEvents!: EmitBridgeEvent[];

  @field(array(struct(GamePlayer)))
  validPlayers!: GamePlayer[];

  constructor(fields: Fields<Effect>) {
    Object.assign(this, fields);
  }

  static fromContext(context: GameContext) {
    const revealed = new Map<Id, Map<number, string>>();
    for (const st of context.randomStates) {
      revealed.set(st.id, st.revealed);
    }
    const answered = new Map<Id, string>();
    for (const st of context.decisionStates) {
      answered.set(st.id, st.value!);
    }
    const actionTimeout = undefined;
    const waitTimeout = undefined;
    const startGame = false;
    const stopGame = false;
    const cancelDispatch = false;
    const timestamp = context.timestamp;
    const currRandomId = context.randomStates.length + 1;
    const currDecisionId = context.decisionStates.length + 1;
    const nodesCount = context.nodes.length;
    const asks: Ask[] = [];
    const assigns: Assign[] = [];
    const releases: Release[] = [];
    const reveals: Reveal[] = [];
    const initRandomStates: RandomSpec[] = [];
    const isCheckpoint = false;
    const settles: Settle[] = [];
    const handlerState = context.handlerState;
    const error = undefined;
    const allowExit = context.allowExit;
    const transfers: Transfer[] = [];
    const launchSubGames: SubGame[] = [];
    const bridgeEvents: EmitBridgeEvent[] = [];
    const validPlayers = context.players;
    return new Effect({
      actionTimeout,
      waitTimeout,
      startGame,
      stopGame,
      cancelDispatch,
      timestamp,
      currRandomId,
      currDecisionId,
      nodesCount,
      asks,
      assigns,
      releases,
      reveals,
      initRandomStates,
      revealed,
      answered,
      isCheckpoint,
      settles,
      handlerState,
      error,
      allowExit,
      transfers,
      launchSubGames,
      bridgeEvents,
      validPlayers
    });
  }
}
