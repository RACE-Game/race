import { PublicKey } from '@solana/web3.js';
import * as borsh from 'borsh';
import { publicKeyExt } from './utils';
import * as RaceCore from '@race-foundation/sdk-core';
import { VoteType } from '@race-foundation/sdk-core';
import { deserialize, serialize, field, option, array, struct } from '@race-foundation/borsh';

export interface IPlayerState {
    isInitialized: boolean;
    nick: string;
    pfpKey?: PublicKey;
}

export interface IPlayerJoin {
    key: PublicKey;
    balance: bigint;
    position: number;
    accessVersion: bigint;
    verifyKey: string;
}

export interface IServerJoin {
    key: PublicKey;
    endpoint: string;
    accessVersion: bigint;
    verifyKey: string;
}

export interface IVote {
    voterKey: PublicKey;
    voteeKey: PublicKey;
    voteType: VoteType;
}

export interface IGameReg {
    title: string;
    gameKey: PublicKey;
    bundleKey: PublicKey;
    regTime: bigint;
}

export interface IRegistryState {
    isInitialized: boolean;
    isPrivate: boolean;
    size: number;
    ownerKey: PublicKey;
    games: IGameReg[];
}

export interface IGameState {
    isInitialized: boolean;
    title: string;
    bundleKey: PublicKey;
    stakeKey: PublicKey;
    ownerKey: PublicKey;
    tokenKey: PublicKey;
    minDeposit: bigint;
    maxDeposit: bigint;
    transactorKey: PublicKey | undefined;
    accessVersion: bigint;
    settleVersion: bigint;
    maxPlayers: number;
    players: IPlayerJoin[];
    servers: IServerJoin[];
    dataLen: number;
    data: Uint8Array;
    votes: IVote[];
    unlockTime: bigint | undefined;
}

export interface IServerState {
    isInitialized: boolean;
    key: PublicKey;
    ownerKey: PublicKey;
    endpoint: string;
}

export class PlayerState implements IPlayerState {
    @field('bool')
    isInitialized!: boolean;
    @field('string')
    nick!: string;
    @field(option(publicKeyExt))
    pfpKey?: PublicKey;

    constructor(fields: IPlayerState) {
        Object.assign(this, fields);
    }

    serialize(): Uint8Array {
        return serialize(this);
    }

    static deserialize(data: Uint8Array): PlayerState {
        return deserialize(PlayerState, data);
    }

    generalize(addr: PublicKey): RaceCore.PlayerProfile {
        return new RaceCore.PlayerProfile({
            addr: addr.toBase58(),
            nick: this.nick,
            pfp: this.pfpKey?.toBase58(),
        });
    }
}

export class Vote implements IVote {
    @field(publicKeyExt)
    voterKey!: PublicKey;
    @field(publicKeyExt)
    voteeKey!: PublicKey;
    @field('u8')
    voteType!: VoteType;
    constructor(fields: IVote) {
        Object.assign(this, fields);
    }
    generalize(): RaceCore.Vote {
        return new RaceCore.Vote({
            voter: this.voterKey.toBase58(),
            votee: this.voteeKey.toBase58(),
            voteType: this.voteType,
        });
    }
}

export class ServerJoin implements IServerJoin {
    @field(publicKeyExt)
    key!: PublicKey;
    @field('string')
    endpoint!: string;
    @field('u64')
    accessVersion!: bigint;
    @field('string')
    verifyKey!: string;
    constructor(fields: IServerJoin) {
        Object.assign(this, fields);
    }
    generalize(): RaceCore.ServerJoin {
        return new RaceCore.ServerJoin({
            addr: this.key.toBase58(),
            endpoint: this.endpoint,
            accessVersion: this.accessVersion,
            verifyKey: this.verifyKey,
        });
    }
}

export class PlayerJoin implements IPlayerJoin {
    @field(publicKeyExt)
    key!: PublicKey;
    @field('u64')
    balance!: bigint;
    @field('u16')
    position!: number;
    @field('u64')
    accessVersion!: bigint;
    @field('string')
    verifyKey!: string;
    constructor(fields: IPlayerJoin) {
        Object.assign(this, fields);
    }
    generalize(): RaceCore.PlayerJoin {
        return new RaceCore.PlayerJoin({
            addr: this.key.toBase58(),
            position: this.position,
            balance: this.balance,
            accessVersion: this.accessVersion,
            verifyKey: this.verifyKey,
        });
    }
}

export class GameState implements IGameState {
    @field('bool')
    isInitialized!: boolean;
    @field('string')
    title!: string;
    @field(publicKeyExt)
    bundleKey!: PublicKey;
    @field(publicKeyExt)
    stakeKey!: PublicKey;
    @field(publicKeyExt)
    ownerKey!: PublicKey;
    @field(publicKeyExt)
    tokenKey!: PublicKey;
    @field('u64')
    minDeposit!: bigint;
    @field('u64')
    maxDeposit!: bigint;
    @field(option(publicKeyExt))
    transactorKey: PublicKey | undefined;
    @field('u64')
    accessVersion!: bigint;
    @field('u64')
    settleVersion!: bigint;
    @field('u16')
    maxPlayers!: number;
    @field(array(struct(PlayerJoin)))
    players!: PlayerJoin[];
    @field(array(struct(ServerJoin)))
    servers!: ServerJoin[];
    @field('u32')
    dataLen!: number;
    @field('u8-array')
    data!: Uint8Array;
    @field(array(struct(Vote)))
    votes!: Vote[];
    @field(option('u64'))
    unlockTime: bigint | undefined;

    constructor(fields: IGameState) {
        Object.assign(this, fields);
    }

    serialize(): Uint8Array {
        return serialize(this);
    }

    static deserialize(data: Uint8Array): GameState {
        return deserialize(GameState, data);
    }

    generalize(addr: PublicKey): RaceCore.GameAccount {
        return new RaceCore.GameAccount({
            addr: addr.toBase58(),
            title: this.title,
            bundleAddr: this.bundleKey.toBase58(),
            ownerAddr: this.ownerKey.toBase58(),
            tokenAddr: this.tokenKey.toBase58(),
            deposits: [],
            minDeposit: this.minDeposit,
            maxDeposit: this.maxDeposit,
            transactorAddr: this.transactorKey?.toBase58(),
            accessVersion: this.accessVersion,
            settleVersion: this.settleVersion,
            maxPlayers: this.maxPlayers,
            players: this.players.map(p => p.generalize()),
            servers: this.servers.map(s => s.generalize()),
            dataLen: this.dataLen,
            data: this.data,
            votes: this.votes.map(v => v.generalize()),
            unlockTime: this.unlockTime,
        });
    }
}

export class GameReg implements IGameReg {
    @field('string')
    title!: string;
    @field(publicKeyExt)
    gameKey!: PublicKey;
    @field(publicKeyExt)
    bundleKey!: PublicKey;
    @field('u64')
    regTime!: bigint;
    constructor(fields: IGameReg) {
        Object.assign(this, fields);
    }
    generalize(): RaceCore.GameRegistration {
        return new RaceCore.GameRegistration({
            title: this.title,
            addr: this.gameKey.toBase58(),
            bundleAddr: this.bundleKey.toBase58(),
            regTime: this.regTime,
        });
    }
}

export class RegistryState implements IRegistryState {
    @field('bool')
    isInitialized!: boolean;
    @field('bool')
    isPrivate!: boolean;
    @field('u16')
    size!: number;
    @field(publicKeyExt)
    ownerKey!: PublicKey;
    @field(array(struct(GameReg)))
    games!: GameReg[];
    constructor(fields: IRegistryState) {
        Object.assign(this, fields);
    }

    serialize(): Uint8Array {
        return serialize(this);
    }

    static deserialize(data: Uint8Array): RegistryState {
        return deserialize(RegistryState, data);
    }

    generalize(addr: PublicKey): RaceCore.RegistrationAccount {
        return new RaceCore.RegistrationAccount({
            addr: addr.toBase58(),
            isPrivate: this.isPrivate,
            size: this.size,
            owner: this.ownerKey.toBase58(),
            games: this.games.map(g => g.generalize()),
        });
    }
}

export class ServerState implements IServerState {
    @field('bool')
    isInitialized!: boolean;
    @field(publicKeyExt)
    key!: PublicKey;
    @field(publicKeyExt)
    ownerKey!: PublicKey;
    @field('string')
    endpoint!: string;

    constructor(fields: IServerState) {
        Object.assign(this, fields);
    }

    serialize(): Uint8Array {
        return serialize(this);
    }

    static deserialize(data: Uint8Array): ServerState {
        return deserialize(this, data);
    }

    generalize(): RaceCore.ServerAccount {
        return new RaceCore.ServerAccount({
            addr: this.ownerKey.toBase58(),
            endpoint: this.endpoint,
        });
    }
}
