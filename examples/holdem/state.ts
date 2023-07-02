export class Player {
  @field('string')
  addr!: string;

  @field('u64')
  chips!: bigint;

  @field('usize')
  position!: number;

  @field('u8')
  status!: number;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

export class ActingPlayer {
  @field('string')
  addr!: string;

  @field('usize')
  position!: number;

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}


export class Pot {
  @field(array('string'))
  owners!: string[];

  @field(array('string'))
  winners!: string[];

  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}


export abstract class Display {}

@variant(0)
export class DealCards extends Display {
  constructor(_: any = {}) {
    super();
  }
}

@variant(1)
export class CollectBets extends Display {
  @field(map('string', 'u64'))
  betMap!: Map<string, bigint>;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(2)
export class UpdateChips extends Display {
  @field('string')
  player!: string;

  @field('u64')
  chips!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(3)
export class AwardPots extends Display {
  @field(array(Pot))
  pots!: Pot[];

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(4)
export class GivePrizes extends Display {
  @field(map('string', 'u64'))
  prizeMap!: Map<string, bigint>;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(5)
export class ShowHoleCards extends Display {
  @field('string')
  player!: string;

  @field(array('usize'))
  cardIdxs!: number[];

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

export class Holdem {
  @field('usize')
  deck_random_id!: number;

  @field('u64')
  sb!: bigint;

  @field('u64')
  bb!: bigint;

  @field('u64')
  minRaise!: bigint;

  @field('usize')
  btn!: number;

  @field('u16')
  rake!: number;

  @field('u8')
  stage!: number;

  @field('u8')
  street!: number;

  @field('u64')
  streetBet!: bigint;

  @field(array('string'))
  board!: string[];

  @field(map('string', array('usize')))
  handIndexMap!: Map<string, number[]>;

  @field(map('string', 'u64'))
  betMap!: Map<string, bigint>;

  @field(map('string', 'u64'))
  prizeMap!: Map<string, bigint>;

  @field(map('string', struct(Player)))
  playerMap!: Map<string, Player>;

  @field(array('string'))
  playerOrder!: string[];

  @field(option(struct(ActingPlayer)))
  actingPlayer!: ActingPlayer | undefined;

  @field(array(struct(Pot)))
  pots!: Pot[];

  @field(array(enum(Display)))
  display!: Display[];

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}
