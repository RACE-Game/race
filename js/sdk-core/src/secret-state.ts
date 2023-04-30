import { IEncryptor } from "./encryptor";

export class SecretState {

  #encryptor: IEncryptor;
  constructor(encryptor: IEncryptor) {
    this.#encryptor = encryptor;
  }

  clear() {

  };

  isRandomLoaded(id: bigint): boolean {
    return true;
  }

  genRandomStates(id: bigint, size: number): any {

  }
}
