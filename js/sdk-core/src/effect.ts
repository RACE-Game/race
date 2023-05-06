export interface IAsk {
  playerAddr: string;
}

export interface IAssign {
  randomId: bigint;
  playerAddr: string;
  indexes: Uint16Array;
}

export interface IReveal {
  randomId: bigint;
  indexes: Uint16Array;
}

export interface IRelease {
  decisionId: bigint;
}

export interface IActionTimeout {
  playerAddr: string;
  timeout: bigint;
}

export class Ask implements IAsk {
  playerAddr!: string;
  constructor(fields: IAsk) {
    Object.assign(this, fields);
  }
}

export class Assign implements IAssign {
  randomId!: bigint;
  playerAddr!: string;
  indexes!: Uint16Array;
  constructor(fields: IAssign) {
    Object.assign(this, fields);
  }
}

export class IReveal {
  randomId!: bigint;
  indexes!: Uint16Array;
  constructor(fields: IReveal) {
    Object.assign(this, fields);
  }
}

export class IRelease {
  decisionId: bigint;
}

export class IActionTimeout {
  playerAddr: string;
  timeout: bigint;
}

export class Effect {
  actionTimeout: ActionTimeout;
}
