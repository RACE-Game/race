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
import { Effect, Settle, SettleAdd, SettleEject, SettleSub } from './effect';
import { GameAccount, PlayerJoin, ServerJoin } from './accounts';
import { Ciphertext, Digest, Id } from './types';

const OPERATION_TIMEOUT = 15_000n;

export type NodeStatus =
    | {
          kind: 'pending';
          accessVersion: bigint;
      }
    | { kind: 'ready' }
    | { kind: 'disconnected' };

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

    constructor(context: GameContext);
    constructor(gameAccount: GameAccount);
    constructor(gameAccountOrContext: GameAccount | GameContext) {
        if (gameAccountOrContext instanceof GameContext) {
            const context = gameAccountOrContext;
            this.gameAddr = context.gameAddr;
            this.accessVersion = context.accessVersion;
            this.settleVersion = context.settleVersion;
            this.transactorAddr = context.transactorAddr;
            this.status = context.status;
            this.players = context.players.map(p => Object.assign({}, p));
            this.servers = context.servers.map(s => Object.assign({}, s));
            this.dispatch = context.dispatch;
            this.handlerState = new Uint8Array(context.handlerState);
            this.timestamp = context.timestamp;
            this.allowExit = context.allowExit;
            this.randomStates = context.randomStates;
            this.decisionStates = context.decisionStates;
            this.settles = context.settles;
        } else {
            const gameAccount = gameAccountOrContext;
            const transactorAddr = gameAccount.transactorAddr;
            if (transactorAddr === undefined) {
                throw new Error('Game not served');
            }
            const players: IPlayer[] = gameAccount.players.map(p => ({
                addr: p.addr,
                balance: p.balance,
                position: p.position,
                status: {
                    kind: 'pending',
                    accessVersion: p.accessVersion,
                },
            }));
            const servers: IServer[] = gameAccount.servers.map(s => ({
                addr: s.addr,
                endpoint: s.endpoint,
                status: {
                    kind: 'pending',
                    accessVersion: s.accessVersion,
                },
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
    }

    getServerByAddress(addr: string): IServer | undefined {
        return this.servers.find(s => s.addr === addr);
    }

    getPlayerByAddress(addr: string): IPlayer | undefined {
        return this.players.find(p => p.addr === addr);
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

    actionTimeout(playerAddr: string, timeout: bigint) {
        this.dispatch = {
            event: new ActionTimeout({ playerAddr }),
            timeout: this.timestamp + timeout,
        };
    }

    genStartGameEvent(): GameEvent {
        return new GameStart({ accessVersion: this.accessVersion });
    }

    startGame() {
        this.randomStates = [];
        this.decisionStates = [];
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

    setPlayerStatus(addr: string, status: NodeStatus) {
        let p = this.players.find(p => p.addr === addr);
        if (p === undefined) {
            throw new Error('Invalid player address');
        }
        p.status = status;
    }

    addPlayer(player: PlayerJoin) {
        const exist = this.players.find(p => p.addr === player.addr || p.position === player.position);
        if (exist === undefined) {
            this.players.push({
                addr: player.addr,
                balance: player.balance,
                status: { kind: 'ready' },
                position: player.position,
            });
        } else {
            if (exist.position === player.position) {
                throw new Error('Position occupied');
            } else {
                throw new Error('Player already joined');
            }
        }
    }

    addServer(server: ServerJoin) {
        const exist = this.players.find(s => s.addr === server.addr);
        if (exist === undefined) {
            this.servers.push({
                addr: server.addr,
                status: { kind: 'ready' },
                endpoint: server.endpoint,
            });
        } else {
            throw new Error('Server already joined');
        }
    }

    setAccessVersion(accessVersion: bigint) {
        this.accessVersion = accessVersion;
    }

    setAllowExit(allowExit: boolean) {
        this.allowExit = allowExit;
    }

    removePlayer(addr: string) {
        if (this.allowExit) {
            const origLen = this.players.length;
            this.players = this.players.filter(p => p.addr !== addr);
            if (this.players.length === origLen) {
                throw new Error('Player not in game');
            }
        } else {
            throw new Error("Can't leave");
        }
    }

    initRandomState(spec: RandomSpec): Id {
        const randomId = this.randomStates.length + 1;
        const owners = this.servers.filter(s => s.status.kind === 'ready').map(s => s.addr);
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
            if (noDispatch) {
                this.dispatchEvent(new OperationTimeout({ addrs: [addr] }), OPERATION_TIMEOUT);
            }
        } else if (statusKind === 'waiting-secrets') {
            if (noDispatch) {
                const addrs = st.listOperatingAddrs();
                this.dispatchEvent(new OperationTimeout({ addrs }), OPERATION_TIMEOUT);
            }
        }
    }

    settle(settles: Settle[]) {
        this.settles = settles;
    }

    bumpSettleVersion() {
        this.settleVersion += 1n;
    }

    applyAndTakeSettles(): Settle[] | undefined {
        if (this.settles === undefined) {
            return undefined;
        }
        let settles = this.settles;
        this.settles = undefined;
        settles = settles.sort((s1, s2) => s1.compare(s2));
        for (const s of settles) {
            if (s.op instanceof SettleAdd) {
                let p = this.getPlayerByAddress(s.addr);
                if (p === undefined) {
                    throw new Error('Invalid settle player address');
                }
                p.balance += s.op.amount;
            } else if (s.op instanceof SettleSub) {
                let p = this.getPlayerByAddress(s.addr);
                if (p === undefined) {
                    throw new Error('Invalid settle player address');
                }
                p.balance -= s.op.amount;
            } else if (s.op instanceof SettleEject) {
                this.players = this.players.filter(p => p.addr !== s.addr);
            }
        }

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
            this.actionTimeout(effect.actionTimeout.playerAddr, effect.actionTimeout.timeout);
        } else if (effect.waitTimeout !== undefined) {
            this.waitTimeout(effect.waitTimeout);
        } else if (effect.cancelDispatch) {
            this.dispatch = undefined;
        }
        this.setAllowExit(effect.allowExit);
        for (const assign of effect.assigns) {
            this.assign(assign.randomId, assign.playerAddr, assign.indexes);
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
        if (effect.settles.length > 0) {
            this.settle(effect.settles);
        }
        if (effect.handlerState !== undefined) {
            this.handlerState = effect.handlerState;
        }
    }

    setNodeReady(accessVersion: bigint) {
        for (const s of this.servers) {
            if (s.status.kind === 'pending') {
                if (s.status.accessVersion < accessVersion) {
                    s.status = { kind: 'ready' };
                }
            }
        }
        for (const p of this.players) {
            if (p.status.kind === 'pending') {
                if (p.status.accessVersion < accessVersion) {
                    p.status = { kind: 'ready' };
                }
            }
        }
    }

    applyCheckpoint(accessVersion: bigint, settleVersion: bigint) {
        if (this.settleVersion !== settleVersion) {
            throw new Error('Invalid checkpoint');
        }
        this.players = this.players.filter(p => {
            if (p.status.kind === 'pending') {
                return p.status.accessVersion <= accessVersion;
            } else {
                return true;
            }
        });
        this.servers = this.servers.filter(s => {
            if (s.status.kind === 'pending') {
                return s.status.accessVersion <= accessVersion;
            } else {
                return true;
            }
        });
        this.accessVersion = accessVersion;
    }
}
