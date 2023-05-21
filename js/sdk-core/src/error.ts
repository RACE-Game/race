import { variant } from '@race/borsh';

export abstract class HandleError {}

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
  }
}

@variant(2)
export class PlayerNotInGame extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(3)
export class CantLeave extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(4)
export class InvalidAmount extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(5)
export class MalformedGameAccountData extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(6)
export class MalformedCustomEvent extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(7)
export class SerializationError extends HandleError {
  constructor(_: any) {
    super();
  }
}

@variant(8)
export class InternalError extends HandleError {
  message: string;
  constructor(fields: { message: string }) {
    super();
    this.message = fields.message;
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
}