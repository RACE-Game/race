import { IEncryptor } from './encryptor';
import { Id } from './types';

export class SecretState {
    #encryptor: IEncryptor;
    constructor(encryptor: IEncryptor) {
        this.#encryptor = encryptor;
    }

    clear() {}

    isRandomLoaded(id: Id): boolean {
        return true;
    }

    genRandomStates(id: Id, size: number): any {}
}
