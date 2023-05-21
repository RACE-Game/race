export type Id = number;
export type Ciphertext = Uint8Array;
export type Secret = Uint8Array;
export type Digest = Uint8Array;
export type Fields<T> = Pick<T, keyof T>;
