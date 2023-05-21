import { GameContext, GameStatus, IPlayer, IServer, NodeStatus } from './game-context';

export class Player {
  readonly addr: string;
  readonly position: number;
  readonly balance: bigint;
  readonly status: NodeStatus;

  constructor(o: IPlayer) {
    this.addr = o.addr;
    this.balance = o.balance;
    this.position = o.position;
    this.status = o.status;
  }
}

export class Server {
  readonly addr: string;
  readonly endpoint: string;
  readonly status: NodeStatus;

  constructor(o: IServer) {
    this.addr = o.addr;
    this.endpoint = o.endpoint;
    this.status = o.status;
  }
}


export class GameContextSnapshot {

  readonly gameAddr: string;
  readonly accessVersion: bigint;
  readonly settleVersion: bigint;
  readonly status: GameStatus;
  readonly allowExit: boolean;
  readonly players: Player[];
  readonly servers: Server[];

  constructor(context: GameContext) {
    this.gameAddr = context.gameAddr;
    this.accessVersion = context.accessVersion;
    this.settleVersion = context.settleVersion;
    this.status = context.status;
    this.allowExit = context.allowExit;
    this.players = context.players.map(p => new Player(p));
    this.servers = context.servers.map(s => new Server(s));
  }
}
