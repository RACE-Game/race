import { RandomState, RandomSpec } from './random-state';
import { DecisionState } from './decision-state';
import {
  ActionTimeout,
  Answer,
  CiphertextAndDigest,
  GameEvent,
  GameStart,
  OperationTimeout,
  Random,
  RandomnessReady,
  SecretShare,
  Shutdown,
  WaitingTimeout,
} from './events';
import { Effect, EmitBridgeEvent, LaunchSubGame, Settle, Transfer } from './effect';
import { GameAccount } from './accounts';
import { Ciphertext, Digest, Id } from './types';

const OPERATION_TIMEOUT = 15_000n;

export type NodeStatus =
  | {
    kind: 'pending';
    accessVersion: bigint;
  }
  | { kind: 'ready' }
  | { kind: 'disconnected' };

export type ClientMode = 'player' | 'transactor' | 'validator';

export type GameStatus = 'idle' | 'running' | 'closed';

export interface INode {
  addr: string;
  id: bigint;
  mode: ClientMode;
  status: NodeStatus;
}

export interface DispatchEvent {
  timeout: bigint;
  event: GameEvent;
}

export interface IdAddrPair {
  id: bigint;
  addr: string;
}

export type SubGame = {
  subId: number;
  bundleAddr: string;
}

export class GameContext {
  gameAddr: string;
  accessVersion: bigint;
  settleVersion: bigint;
  transactorAddr: string;
  status: GameStatus;
  nodes: INode[];
  dispatch: DispatchEvent | undefined;
  handlerState: Uint8Array;
  timestamp: bigint;
  allowExit: boolean;
  randomStates: RandomState[];
  decisionStates: DecisionState[];
  settles: Settle[] | undefined;
  transfers: Transfer[] | undefined;
  checkpoint: Uint8Array | undefined;
  checkpointAccessVersion: bigint;
  launchSubGames: LaunchSubGame[];
  bridgeEvents: EmitBridgeEvent[];

  constructor(context: GameContext);
  constructor(gameAccount: GameAccount);
  constructor(init: GameAccount | GameContext) {
    if (init instanceof GameContext) {
      const context = init;
      this.gameAddr = context.gameAddr;
      this.accessVersion = context.accessVersion;
      this.settleVersion = context.settleVersion;
      this.transactorAddr = context.transactorAddr;
      this.status = context.status;
      this.nodes = context.nodes.map(n => Object.assign({}, n));
      this.dispatch = context.dispatch;
      this.handlerState = new Uint8Array(context.handlerState);
      this.timestamp = context.timestamp;
      this.allowExit = context.allowExit;
      this.randomStates = context.randomStates;
      this.decisionStates = context.decisionStates;
      this.settles = context.settles;
      this.transfers = context.transfers;
      this.checkpoint = undefined;
      this.checkpointAccessVersion = context.checkpointAccessVersion;
      this.launchSubGames = context.launchSubGames.map(sg => Object.assign({}, sg));
      this.bridgeEvents = context.bridgeEvents;
    } else {
      const gameAccount = init;
      const transactorAddr = gameAccount.transactorAddr;
      if (transactorAddr === undefined) {
        throw new Error('Game not served');
      }
      let nodes: INode[] = [];
      gameAccount.servers.forEach(s => nodes.push({
        addr: s.addr,
        id: s.accessVersion,
        mode: s.addr === transactorAddr ? 'transactor' : 'validator',
        status: s.addr === gameAccount.transactorAddr
          ? { kind: 'ready' }
          : {
            kind: 'pending',
            accessVersion: s.accessVersion,
          },
      }));
      gameAccount.players.forEach(p => nodes.push({
        addr: p.addr,
        id: p.accessVersion,
        mode: 'player',
        status: p.addr === gameAccount.transactorAddr
          ? { kind: 'ready' }
          : {
            kind: 'pending',
            accessVersion: p.accessVersion,
          },
      }))

      this.gameAddr = gameAccount.addr;
      this.transactorAddr = transactorAddr;
      this.accessVersion = gameAccount.accessVersion;
      this.settleVersion = gameAccount.settleVersion;
      this.status = 'idle';
      this.dispatch = undefined;
      this.nodes = nodes;
      this.timestamp = 0n;
      this.allowExit = false;
      this.randomStates = [];
      this.decisionStates = [];
      this.settles = undefined;
      this.transfers = undefined;
      this.handlerState = Uint8Array.of();
      this.checkpoint = undefined;
      this.checkpointAccessVersion = gameAccount.checkpointAccessVersion;
      this.launchSubGames = [];
      this.bridgeEvents = [];
    }
  }

  subContext(launchSubGame: LaunchSubGame): GameContext {
    const c = new GameContext(this);
    c.gameAddr = c.gameAddr + launchSubGame.subId;
    c.dispatch = undefined;
    c.timestamp = 0n;
    c.allowExit = false;
    c.randomStates = [];
    c.decisionStates = [];
    c.settles = undefined;
    c.transfers = undefined;
    c.handlerState = Uint8Array.of();
    c.checkpoint = launchSubGame.checkpoint;
    c.launchSubGames = [];
    c.bridgeEvents = [];
    return c;
  }

  idToAddrUnchecked(id: bigint): string | undefined {
    return this.nodes.find(x => x.id === id)?.addr;
  }

  idToAddr(id: bigint): string {
    let found = this.idToAddrUnchecked(id);
    if (found === undefined) {
      throw new Error(`Cannot map id to address: ${id.toString()}`);
    }
    return found;
  }

  addrToIdUnchecked(addr: string): bigint | undefined {
    return this.nodes.find(x => x.addr === addr)?.id;
  }

  addrToId(addr: string): bigint {
    let found = this.addrToIdUnchecked(addr);
    if (found === undefined) {
      throw new Error(`Cannot map address to id: ${addr}`);
    }
    return found;
  }

  getNodeByAddress(addr: string): INode | undefined {
    return this.nodes.find(n => n.addr === addr);
  }

  dispatchEvent(event: GameEvent, timeout: bigint) {
    this.dispatch = {
      event,
      timeout: this.timestamp + timeout,
    };
  }

  dispatchEventInstantly(event: GameEvent) {
    this.dispatchEvent(event, 0n);
  }

  waitTimeout(timeout: bigint) {
    this.dispatch = {
      event: new WaitingTimeout({}),
      timeout: this.timestamp + timeout,
    };
  }

  actionTimeout(playerId: bigint, timeout: bigint) {
    this.dispatch = {
      event: new ActionTimeout({ playerId }),
      timeout: this.timestamp + timeout,
    };
  }

  genStartGameEvent(): GameEvent {
    return new GameStart({ accessVersion: this.accessVersion });
  }

  startGame() {
    this.randomStates = [];
    this.dispatch = {
      event: this.genStartGameEvent(),
      timeout: 0n,
    };
  }

  shutdownGame() {
    this.dispatch = {
      event: new Shutdown({}),
      timeout: 0n,
    };
  }

  getRandomState(randomId: Id): RandomState {
    if (randomId <= 0) {
      throw new Error('Invalid random id: ' + randomId);
    }
    const st = this.randomStates[randomId - 1];
    if (st === undefined) {
      throw new Error('Invalid random id: ' + randomId);
    }
    return st;
  }

  getDecisionState(decisionId: Id): DecisionState {
    if (decisionId <= 0) {
      throw new Error('Invalid decision id: ' + decisionId);
    }
    const st = this.decisionStates[decisionId - 1];
    if (st === undefined) {
      throw new Error('Invalid decision id: ' + decisionId);
    }
    return st;
  }

  assign(randomId: Id, playerAddr: string, indexes: number[]) {
    const st = this.getRandomState(randomId);
    st.assign(playerAddr, indexes);
  }

  reveal(randomId: Id, indexes: number[]) {
    const st = this.getRandomState(randomId);
    st.reveal(indexes);
  }

  isRandomReady(randomId: Id): boolean {
    const k = this.getRandomState(randomId).status.kind;
    return k === 'ready' || k === 'waiting-secrets';
  }

  isAllRandomReady(): boolean {
    for (const st of this.randomStates) {
      const k = st.status.kind;
      if (k !== 'ready' && k !== 'waiting-secrets') {
        return false;
      }
    }
    return true;
  }

  isSecretsReady(): boolean {
    return this.randomStates.every(st => st.status.kind === 'ready');
  }

  setNodeStatus(addr: string, status: NodeStatus) {
    let n = this.nodes.find(n => n.addr === addr);
    if (n === undefined) {
      throw new Error('Invalid node address');
    }
    n.status = status;
  }

  addNode(nodeAddr: string, accessVersion: bigint, mode: ClientMode) {
    this.nodes = this.nodes.filter(n => n.addr !== nodeAddr);
    this.nodes.push({
      addr: nodeAddr,
      id: accessVersion,
      mode,
      status: { kind: 'pending', accessVersion }
    });
  }

  setAccessVersion(accessVersion: bigint) {
    this.accessVersion = accessVersion;
  }

  setAllowExit(allowExit: boolean) {
    this.allowExit = allowExit;
  }

  initRandomState(spec: RandomSpec): Id {
    const randomId = this.randomStates.length + 1;
    const owners = this.nodes.filter(n => n.status.kind === 'ready' && n.mode !== 'player').map(n => n.addr);
    const randomState = new RandomState(randomId, spec, owners);
    this.randomStates.push(randomState);
    return randomId;
  }

  addSharedSecrets(_addr: string, shares: SecretShare[]) {
    for (const share of shares) {
      if (share instanceof Random) {
        const { randomId, toAddr, fromAddr, index, secret } = share;
        this.getRandomState(randomId).addSecret(fromAddr, toAddr, index, secret);
      } else if (share instanceof Answer) {
        const { fromAddr, decisionId, secret } = share;
        this.getDecisionState(decisionId).addSecret(fromAddr, secret);
      }
    }
  }

  randomizeAndMask(addr: string, randomId: Id, ciphertexts: Ciphertext[]) {
    let st = this.getRandomState(randomId);
    st.mask(addr, ciphertexts);
    this.dispatchRandomizationTimeout(randomId);
  }

  lock(addr: string, randomId: Id, ciphertextsAndTests: CiphertextAndDigest[]) {
    let st = this.getRandomState(randomId);
    st.lock(addr, ciphertextsAndTests);
    this.dispatchRandomizationTimeout(randomId);
  }

  dispatchRandomizationTimeout(randomId: Id) {
    const noDispatch = this.dispatch === undefined;
    let st = this.getRandomState(randomId);
    const statusKind = st.status.kind;
    if (statusKind === 'ready') {
      this.dispatchEventInstantly(new RandomnessReady({ randomId }));
    } else if (statusKind === 'locking' || statusKind === 'masking') {
      const addr = st.status.addr;
      const id = this.addrToId(addr);
      if (noDispatch) {
        this.dispatchEvent(new OperationTimeout({ ids: [id] }), OPERATION_TIMEOUT);
      }
    } else if (statusKind === 'waiting-secrets') {
      if (noDispatch) {
        const ids = st.listOperatingAddrs().map(x => this.addrToId(x));
        this.dispatchEvent(new OperationTimeout({ ids }), OPERATION_TIMEOUT);
      }
    }
  }

  settle(settles: Settle[]) {
    this.settles = settles;
  }

  transfer(transfers: Transfer[]) {
    this.transfers = transfers;
  }

  bumpSettleVersion() {
    this.settleVersion += 1n;
  }

  /*
  This function refers to the backend function `take_settles_and_transfers`.
  Here, we don't have to deal with transfers before we introducing settlement validation.
   */
  applyAndTakeSettles(): Settle[] | undefined {
    if (this.settles === undefined) {
      return undefined;
    }
    let settles = this.settles;
    this.settles = undefined;
    settles = settles.sort((s1, s2) => s1.compare(s2));
    this.bumpSettleVersion();
    return settles;
  }

  addSettle(settle: Settle) {
    if (this.settles === undefined) {
      this.settles = [settle];
    } else {
      this.settles.push(settle);
    }
  }

  addRevealedRandom(randomId: Id, revealed: Map<number, string>) {
    const st = this.getRandomState(randomId);
    st.addRevealed(revealed);
  }

  addRevealedAnswer(decisionId: Id, revealed: string) {
    const st = this.getDecisionState(decisionId);
    st.addReleased(revealed);
  }

  ask(owner: string): Id {
    const id = this.decisionStates.length + 1;
    const st = new DecisionState(id, owner);
    this.decisionStates.push(st);
    return id;
  }

  answerDecision(id: Id, owner: string, ciphertext: Ciphertext, digest: Digest) {
    const st = this.getDecisionState(id);
    st.setAnswer(owner, ciphertext, digest);
  }

  getRevealed(randomId: Id): Map<number, string> {
    let st = this.getRandomState(randomId);
    return st.revealed;
  }

  applyEffect(effect: Effect) {
    if (effect.startGame) {
      this.startGame();
    } else if (effect.stopGame) {
      this.shutdownGame();
    } else if (effect.actionTimeout !== undefined) {
      this.actionTimeout(effect.actionTimeout.playerId, effect.actionTimeout.timeout);
    } else if (effect.waitTimeout !== undefined) {
      this.waitTimeout(effect.waitTimeout);
    } else if (effect.cancelDispatch) {
      this.dispatch = undefined;
    }
    this.setAllowExit(effect.allowExit);
    for (const assign of effect.assigns) {
      const addr = this.idToAddr(assign.playerId);
      this.assign(assign.randomId, addr, assign.indexes);
    }
    for (const reveal of effect.reveals) {
      this.reveal(reveal.randomId, reveal.indexes);
    }
    for (const ask of effect.asks) {
      this.ask(ask.playerAddr);
    }
    for (const spec of effect.initRandomStates) {
      this.initRandomState(spec);
    }
    if (effect.isCheckpoint) {
      this.settle(effect.settles);
      this.transfer(effect.transfers);
      this.checkpoint = effect.checkpoint;
      this.status = 'idle';
    }
    if (effect.handlerState !== undefined) {
      this.handlerState = effect.handlerState;
    }
    this.launchSubGames.push(...effect.launchSubGames);
    this.bridgeEvents = effect.bridgeEvents;
  }

  setNodeReady(accessVersion: bigint) {
    for (const n of this.nodes) {
      if (n.status.kind === 'pending') {
        if (n.status.accessVersion <= accessVersion) {
          console.debug(`Set node ${n.addr} status to ready`);
          n.status = { kind: 'ready' };
        }
      }
    }
  }

  applyCheckpoint(accessVersion: bigint, settleVersion: bigint) {
    console.log(`Apply checkpoint, accessVersion: ${accessVersion}`)
    if (this.settleVersion !== settleVersion) {
      throw new Error(`Invalid checkpoint, local settle version: ${this.settleVersion}, remote settle version: ${settleVersion}`);
    }
    this.accessVersion = accessVersion;
  }

  prepareForNextEvent(timestamp: bigint) {
    this.timestamp = timestamp;
    this.checkpoint = undefined;
  }

  findSubGame(subId: number): LaunchSubGame | undefined {
    return this.launchSubGames.find(g => g.subId === Number(subId));
  }
}
