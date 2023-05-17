import { variant } from '@race/borsh';

export abstract class HandlerError { }

@variant(0)
export class CustomError extends HandlerError {
  message: string;
  constructor(fields: { message: string }) {
    super();
    this.message = fields.message;
  }
}

@variant(1)
export class NoEnoughPlayers extends HandlerError {
  constructor(_: any) { super() }
}

@variant(2)
export class PlayerNotInGame extends HandlerError {
  constructor(_: any) { super() }
}

@variant(3)
export class CantLeave extends HandlerError {
  constructor(_: any) { super() }
}

@variant(4)
export class InvalidAmount extends HandlerError {
  constructor(_: any) { super() }
}

@variant(5)
export class MalformedGameAccountData extends HandlerError {
  constructor(_: any) { super() }
}

@variant(6)
export class MalformedCustomEvent extends HandlerError {
  constructor(_: any) { super() }
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
}
