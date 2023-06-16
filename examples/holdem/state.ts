
class Bet {
  @field('string')
  owner!: string;

  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

const PLAYER_STATUS = {
  WAIT: 0,
  ACTED: 1,
  ACTING: 2,
  ALLIN: 3,
  FOLD: 4
};

type PlayerStatus = typeof PLAYER_STATUS[typeof keyof PLAYER_STATUS];

class Player {
  @field('string')
  addr!: string;

  @field('u64')
  chips!: bigint;

  @field('usize')
  position!: number;

  @field('u8')
  status!: PlayerStatus;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

class Pot {
  @field(vec('string'))
  owners!: string[];

  @field(vec('string'))
  winners!: string[];

  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

const STREET = {
  INIT: 0,
  PREFLOP: 1,
  FLOP: 2,
  TURN: 3,
  RIVER: 4,
  SHOWDOWN: 5
};

type Street = typeof STREET[typeof keyof STREET];

class HoldemAccount {
  @field('u64')
  sb!: bigint;

  @field('u64')
  bb!: bigint;

  @field('u16')
  rake!: number;

  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

abstract class GameEvent {}

@variant(0)
class Bet {
  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields)
  }
}

@variant(1)
class Check {
  constructor(_: any) {
    super();
  }
}


@variant(2)
class Call {
  constructor(_: any) {
    super();
  }
}


@variant(3)
class Fold {
  constructor(_: any) {
    super();
  }
}


@variant(4)
class Raise {
  @field('u64')
  amount!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields)
  }
}

const HOLDEM_STAGE = {
  INIT: 0,
  SHARE_KEY: 1,
  PLAY: 2,
  RUNNER: 3,
  SETTLE: 4,
  SHOWDOWN: 5
};

type HoldemStage = typeof HOLDEM_STAGE[typeof keyof HOLDEM_STAGE];

class ActingPlayer {
  @field('string')
  addr!: string;

  @field('usize')
  position!: number;
}

class Holdem {
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
  stage!: HoldemStage;

  @field('u8')
  street!: Street;

  @field('u64')
  streetBet!: bigint;

  @field(vec('string'))
  board!: string[];

  @field(map('string', struct(Bet)))
  betMap!: Map<string, Bet>;

  @field(map('string', 'u64'))
  prizeMap!: Map<string, bigint>;

  @field(map('string', struct(Player)))
  playerMap!: Map<string, Player>;

  @field(option(struct(ActingPlayer)))
  actingPlayer!: ActingPlayer | undefined;

  @field(vec(struct(Pot)))
  pots!: Pot[];

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}
