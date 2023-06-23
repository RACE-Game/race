export class Bet {
  @field('string')
  owner!: string;

  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

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

export class ActingPlayer {
  @field('string')
  addr!: string;

  @field('usize')
  position!: number;

  constructor(fields: any) {
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

  @field(map('string', struct(Bet)))
  betMap!: Map<string, Bet>;

  @field(map('string', 'u64'))
  prizeMap!: Map<string, bigint>;

  @field(map('string', struct(Player)))
  playerMap!: Map<string, Player>;

  @field(array('string'))
  players!: string[];

  @field(option(struct(ActingPlayer)))
  actingPlayer!: ActingPlayer | undefined;

  @field(array(struct(Pot)))
  pots!: Pot[];

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}
