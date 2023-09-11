import { field, variant } from '@race-foundation/borsh';

export abstract class HandleError extends Error {}

@variant(0)
export class CustomError extends HandleError {
  message: string;
  constructor(fields: { message: string }) {
    super();
    this.message = fields.message;
  }
}

@variant(1)
export class NoEnoughPlayers extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'No enough players';
  }
}

@variant(2)
export class PlayerNotInGame extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'Player not in game';
  }
}

@variant(3)
export class CantLeave extends HandleError {
  constructor(_: any) {
    super();
    this.message = "Can't leave game";
  }
}

@variant(4)
export class InvalidAmount extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'Invalid amount';
  }
}

@variant(5)
export class MalformedGameAccountData extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'Malformed game account data';
  }
}

@variant(6)
export class MalformedCustomEvent extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'Malformed custom event';
  }
}

@variant(7)
export class SerializationError extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'Serilization error';
  }
}

@variant(8)
export class NoEnoughServers extends HandleError {
  constructor(_: any) {
    super();
    this.message = 'No enough servers';
  }
}

@variant(9)
export class InternalError extends HandleError {
  @field('string')
  message: string;
  constructor(fields: { message: string }) {
    super();
    this.message = `Internal error: ${fields.message}`;
  }
}

@variant(10)
export class CheckpointWithoutSettleError extends HandleError {
  constructor(_: any) {
    super();
  }
}

export class SdkError extends Error {
  constructor(message: string) {
    super(message);
  }

  static publicKeyNotFound(addr: string) {
    return new SdkError(`RSA public key for ${addr} is missing`);
  }

  static gameAccountNotFound(addr: string) {
    return new SdkError(`Game account of ${addr} not found`);
  }

  static gameBundleNotFound(addr: string) {
    return new SdkError(`Game bundle of ${addr} not found`);
  }

  static transactorAccountNotFound(addr: string) {
    return new SdkError(`Transactor's account of ${addr} not found`);
  }

  static gameNotServed(addr: string) {
    return new SdkError(`Game at ${addr} is not served`);
  }

  static tokenNotFound(addr: string) {
    return new SdkError(`Token ${addr} not found`);
  }
}
