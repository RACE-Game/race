import * as borsh from 'borsh';

export interface IPlayerJoin {
  readonly addr: string;
  readonly position: number;
  readonly balance: bigint;
  readonly accessVersion: bigint;
};

export type PlayerDeposit = {
  readonly addr: string;
  readonly amount: bigint;
  readonly settleVersion: bigint;
};

export type ServerJoin = {
  readonly addr: string;
  readonly endpoint: string;
  readonly accessVersion: bigint;
};

export enum VoteType {
  ServerVoteTransactorDropOff = 0,
  ClientVoteTransactorDropOff = 1,
}

export type Vote = {
  readonly voter: string;
  readonly votee: string;
  readonly voteType: VoteType;
};

export type GameRegistration = {
  readonly title: string;
  readonly addr: string;
  readonly regTime: bigint;
  readonly bundleAddr: string;
};

export type GameAccount = {
  readonly addr: string;
  readonly title: string;
  readonly bundleAddr: string;
  readonly tokenAddr: string;
  readonly ownerAddr: string;
  readonly settleVersion: bigint;
  readonly accessVersion: bigint;
  readonly players: PlayerJoin[];
  readonly deposits: PlayerDeposit[];
  readonly servers: ServerJoin[];
  readonly transactorAddr: string | undefined;
  readonly votes: Vote[];
  readonly unlockTime: bigint | undefined;
  readonly maxPlayers: number;
  readonly dataLen: number;
  readonly data: number[];
  readonly minDeposit: bigint;
  readonly maxDeposit: bigint;
};

export type ServerAccount = {
  readonly addr: string;
  readonly endpoint: string;
};

export type GameBundle = {
  readonly uri: string;
  readonly name: string;
  readonly data: number[];
};

export type PlayerProfile = {
  readonly addr: string;
  readonly nick: string;
  readonly pfp: string | undefined;
};

export type RegistrationAccount = {
  readonly addr: string;
  readonly isPrivate: boolean;
  readonly size: number;
  readonly owner: string | undefined;
  readonly games: GameRegistration[];
};

export class PlayerJoin implements IPlayerJoin {
  readonly addr!: string;
  readonly position!: number;
  readonly balance!: bigint;
  readonly accessVersion!: bigint;
  constructor(fields: IPlayerJoin) {
    Object.assign(this, fields)
  }
};

const playerJoinSchema = new Map([
  [
    PlayerJoin, {
      kind: 'struct',
      fields: [
        ['addr', 'string'],
        ['position', 'u32'],
        ['balance', 'bigint'],
        ['accessVersion', 'bigint']
      ]
    }
  ]
]);
