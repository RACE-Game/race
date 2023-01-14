import { Address, Amount, Position, Timestamp, Version } from "./common";

export type PlayerJoin = {
    addr: Address,
    position: Position,
    accessVersion: Version,
};

export type PlayerDeposit = {
    addr: Address,
    amount: Amount,
    accessVersion: Version,
}

export type ServerAccount = {
    addr: Address,
    ownerAddr: Address,
    endpoint: string
}

export type GameAccount = {
    addr: Address,
    bundleAddr: Address
    settleVersion: Version,
    accessVersion: Version,
    players: PlayerJoin[],
    deposits: PlayerDeposit[],
    serverAddrs: Address[],
    transactorAddr: Address | null,
    maxPlayers: number,
    dataLen: number,
    data: Uint8Array,
}

export type GameRegistration = {
    addr: Address,
    regTime: Timestamp,
    bundleAddr: Address
}

export type RegistrationAccount = {
    addr: Address,
    isPrivate: boolean,
    size: number,
    owner: Address | null,
    games: GameRegistration[],
}

export type GameBundle = {
    addr: Address,
    data: Uint8Array,
}

export type PlayerProfile = {
    addr: Address,
    pfp: Address,
    data: Uint8Array
}
