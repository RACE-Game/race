import { RandomState, RandomSpec } from './random-state';
import { DecisionState } from './decision-state';
import { ActionTimeout, GameEvent, GameStart, SecretShare, Shutdown, WaitingTimeout } from './events';
import { Effect, Settle } from './effect';
import { GameAccount, PlayerJoin, ServerJoin } from './accounts';

export type NodeStatus = {
  kind: 'pending',
  accessVersion: bigint,
} | { kind: 'ready' } | { kind: 'disconnected' };

export type GameStatus = 'uninit' | 'running' | 'closed';

export interface IPlayer {
  addr: string;
  position: number;
  balance: bigint;
  status: NodeStatus;
}

export interface IServer {
  addr: string;
  status: NodeStatus;
  endpoint: string;
}

export interface DispatchEvent {
  timeout: bigint;
  event: GameEvent;
}

export class GameContext {
  gameAddr: string;
  accessVersion: bigint;
  settleVersion: bigint;
  transactorAddr: string;
  status: GameStatus;
  players: IPlayer[];
  servers: IServer[];
  dispatch: DispatchEvent | undefined;
  handlerState: Uint8Array;
  timestamp: bigint;
  allowExit: boolean;
  randomStates: RandomState[];
  decisionStates: DecisionState[];
  settles: Settle[] | undefined;

  constructor(gameAccount: GameAccount) {
    const transactorAddr = gameAccount.transactorAddr;
    if (transactorAddr === undefined) {
      throw new Error('Game not served');
    }
    const players: IPlayer[] = gameAccount
      .players
      .map(p => ({
        addr: p.addr,
        balance: p.balance,
        position: p.position,
        status: {
          kind: 'pending',
          accessVersion: p.accessVersion
        }
      }));
    const servers: IServer[] = gameAccount
      .servers
      .map(s => ({
        addr: s.addr,
        endpoint: s.endpoint,
        status: {
          kind: 'pending',
          accessVersion: s.accessVersion
        }
      }));

    this.gameAddr = gameAccount.addr;
    this.transactorAddr = transactorAddr;
    this.accessVersion = gameAccount.accessVersion;
    this.settleVersion = gameAccount.settleVersion;
    this.status = 'uninit';
    this.dispatch = undefined;
    this.players = players;
    this.servers = servers;
    this.timestamp = 0n;
    this.allowExit = false;
    this.randomStates = [];
    this.decisionStates = [];
    this.settles = undefined;
    this.handlerState = Uint8Array.of();
  }


  getServerByAddress(addr: string): IServer | undefined {
    return this.servers.find(s => s.addr === addr);
  }

  getPlayerByAddress(addr: string): IPlayer | undefined {
    return this.players.find(p => p.addr === addr);
  }

  dispatchEvent(event: GameEvent, timeout: bigint) {
    this.dispatch = {
      event, timeout: this.timestamp + timeout
    };
  }

  waitTimeout(timeout: bigint) {
    this.dispatch = {
      event: new WaitingTimeout({}),
      timeout: this.timestamp + timeout,
    }
  }

  actionTimeout(playerAddr: string, timeout: bigint) {
    this.dispatch = {
      event: new ActionTimeout({ playerAddr }),
      timeout: this.timestamp + timeout,
    }
  }

  genStartGameEvent(): GameEvent {
    return new GameStart({ accessVersion: this.accessVersion })
  }

  startGame() {
    this.randomStates = [];
    this.decisionStates = [];
    this.dispatch = {
      event: this.genStartGameEvent(),
      timeout: 0n
    };
  }

  shutdownGame() {
    this.dispatch = {
      event: new Shutdown({}),
      timeout: 0n
    };
  }

  assign(randomId: bigint, playerAddr: string, indexes: number[]) {

  }

  reveal(randomId: bigint, indexes: number[]) {

  }


  isRandomReady(randomId: bigint): boolean {
    return true;
  }

  isAllRandomReady(): boolean {
    return true;
  }

  isSecretsReady(): boolean {
    return true;
  }

  setPlayerStatus(addr: string, status: NodeStatus) {

  }

  addPlayer(player: PlayerJoin) {

  }

  addServer(server: ServerJoin) {

  }

  initRandomState(spec: RandomSpec): bigint {
    return 0n;
  }

  addSharedSecrets(_addr: string, shares: SecretShare[]) {
  }

  dispatchRandomizationTimeout(randomId: bigint) {

  }

  settle(settles: Settle[]) {

  }

  bumpSettleVersion() {

  }

  applyAndTakeSettles(): Settle[] | undefined {
    return undefined;
  }

  applyEffect(effect: Effect) {

  }

  setNodeReady(accessVersion: bigint) {

  }

  applyCheckpoint(accessVersion: bigint, settleVersion: bigint) {

  }
}
