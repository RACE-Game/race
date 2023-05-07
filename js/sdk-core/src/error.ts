export class SdkError extends Error {

  constructor(message: string) {
    super(message);
  }

  static publicKeyNotFound(addr: string) {
    return new SdkError(`RSA public key for ${addr} is missing`);
  }
}
