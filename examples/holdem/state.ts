export class Player {
  @field('string')
  addr!: string;

  @field('u64')
  chips!: bigint;

  @field('usize')
  position!: number;

  @field('u8')
  status!: number;

  @field('u8')
  timeout!: number;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

export class ActingPlayer {
  @field('string')
  addr!: string;

  @field('usize')
  position!: number;

  @field('u64')
  clock!: bigint;

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

export class AwardPot {
  @field(array('string'))
  winners!: string[];

  @field('u64')
  amount!: bigint;
}

export abstract class Display {}

@variant(0)
export class DealCards extends Display {
  constructor(_: any = {}) {
    super();
  }
}

@variant(1)
export class DealBoard extends Display {
  @field('usize')
  prev!: number;

  @field(array('string'))
  board!: string[];

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(2)
export class CollectBets extends Display {
  @field(map('string', 'u64'))
  betMap!: Map<string, bigint>;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(3)
export class UpdateChips extends Display {
  @field('string')
  player!: string;

  @field('u64')
  before!: bigint;

  @field('u64')
  after!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(4)
export class AwardPots extends Display {
  @field(array(AwardPot))
  pots!: AwardPot[];

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

  @field(array(struct(Pot)))
  pots!: Pot[];

  @field(option(struct(ActingPlayer)))
  actingPlayer!: ActingPlayer | undefined;

  @field(array(enum(Display)))
  display!: Display[];

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}
