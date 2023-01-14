export type Address = string;
export type Amount = bigint;
export type Position = number;
export type Version = bigint;
export type Ciphertext = Uint8Array;
export type RandomId = string;
export type SecretDigest = Uint8Array;
export type SecretKey = Uint8Array;
export type Timestamp = bigint;

export type ClientMode = "player" | "transactor" | "validator";
export type RandomMode = "shuffler" | "drawer";
export type Chain = "facade" | "solana" | "bnb";

export type SecretIdent = {
    fromAddr: Address,
    toAddr: Address | null,
    randomId: RandomId,
    index: number
};
