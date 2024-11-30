import { IEncryptor } from './encryptor'

export class SecretState {
    #encryptor: IEncryptor
    constructor(encryptor: IEncryptor) {
        this.#encryptor = encryptor
    }

    clear() {}

    isRandomLoaded(id: number): boolean {
        return true
    }

    genRandomStates(id: number, size: number): any {}
}
