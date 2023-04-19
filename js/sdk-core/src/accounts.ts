export type PlayerJoin = {
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
};

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
  addr: string;
  title: string;
  bundleAddr: string;
  tokenAddr: string;
  ownerAddr: string;
  settleVersion: bigint;
  accessVersion: bigint;
  players: PlayerJoin[];
  deposits: PlayerDeposit[];
  servers: ServerJoin[];
  transactorAddr: string | undefined;
  votes: Vote[];
  unlockTime: bigint | undefined;
  maxPlayers: number;
  dataLen: number;
  data: Uint8Array;
  minDeposit: bigint;
  maxDeposit: bigint;
};

export type ServerAccount = {
  readonly addr: string;
  readonly ownerAddr: string;
  readonly endpoint: string;
};

export type GameBundle = {
  readonly uri: string;
  readonly name: string;
  readonly data: Uint8Array;
};

export type PlayerProfile = {
  readonly addr: string;
  readonly nick: string;
  readonly pfp: string | undefined;
};

export type RegistrationAccount = {
  readonly isPrivate: boolean;
  readonly size: number;
  readonly owner: string | undefined;
  readonly games: GameRegistration[];
};
