import { RandomState, RandomSpec } from './random-state'
import { DecisionState } from './decision-state'
import { Checkpoint } from './checkpoint'
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
} from './events'
import { InitAccount } from './init-account'
import { Effect, EmitBridgeEvent, SubGame, Settle, Transfer } from './effect'
import { EntryType, EntryTypeDisabled, GameAccount } from './accounts'
import { Ciphertext, Digest, Fields, Id } from './types'
import { clone } from './utils'
import rfdc from 'rfdc'
import { sha256String } from './encryptor'

const OPERATION_TIMEOUT = 15_000n

export type NodeStatus =
  | {
      kind: 'pending'
      accessVersion: bigint
    }
  | { kind: 'ready' }
  | { kind: 'disconnected' }

export type ClientMode = 'player' | 'transactor' | 'validator'

export type GameStatus = 'idle' | 'running' | 'closed'

export interface INode {
  addr: string
  id: bigint
  mode: ClientMode
  status: NodeStatus
}

export interface DispatchEvent {
  timeout: bigint
  event: GameEvent
}

export interface IdAddrPair {
  id: bigint
  addr: string
}

export type EventEffects = {
  settles: Settle[]
  transfers: Transfer[]
  checkpoint: Uint8Array | undefined
  launchSubGames: SubGame[]
  bridgeEvents: EmitBridgeEvent[]
  startGame: boolean
}

export class ContextPlayer {
  id!: bigint
  position!: number
  constructor(fields: Fields<ContextPlayer>) {
    Object.assign(this, fields)
  }
}

export class GameContext {
  gameAddr: string
  gameId: number
  accessVersion: bigint
  settleVersion: bigint
  status: GameStatus
  nodes: INode[]
  dispatch: DispatchEvent | undefined
  handlerState: Uint8Array
  timestamp: bigint
  randomStates: RandomState[]
  decisionStates: DecisionState[]
  checkpoint: Checkpoint
  subGames: SubGame[]
  initData: Uint8Array
  maxPlayers: number
  players: ContextPlayer[]
  entryType: EntryType
  stateSha: string

  constructor(gameAccount: GameAccount, checkpoint: Checkpoint) {
    if (checkpoint === undefined) {
      throw new Error('Missing checkpoint')
    }
    console.info('Build game context with checkpoint:', clone(checkpoint))
    const checkpointAccessVersion = gameAccount.checkpointOnChain?.accessVersion || 0
    const transactorAddr = gameAccount.transactorAddr
    if (transactorAddr === undefined) {
      throw new Error('Game not served')
    }
    let nodes: INode[] = []
    gameAccount.servers.forEach(s =>
      nodes.push({
        addr: s.addr,
        id: s.accessVersion,
        mode: s.addr === transactorAddr ? 'transactor' : 'validator',
        status:
          s.addr === gameAccount.transactorAddr
            ? { kind: 'ready' }
            : {
                kind: 'pending',
                accessVersion: s.accessVersion,
              },
      })
    )
    gameAccount.players.forEach(p =>
      nodes.push({
        addr: p.addr,
        id: p.accessVersion,
        mode: 'player',
        status:
          p.addr === gameAccount.transactorAddr
            ? { kind: 'ready' }
            : {
                kind: 'pending',
                accessVersion: p.accessVersion,
              },
      })
    )

    const players = gameAccount.players
      .filter(p => p.accessVersion <= checkpointAccessVersion)
      .map(
        p =>
          new ContextPlayer({
            id: p.accessVersion,
            position: p.position,
          })
      )

    this.gameAddr = gameAccount.addr
    this.gameId = 0
    this.accessVersion = gameAccount.accessVersion
    this.settleVersion = gameAccount.settleVersion
    this.status = 'idle'
    this.dispatch = undefined
    this.nodes = nodes
    this.timestamp = 0n
    this.randomStates = []
    this.decisionStates = []
    this.handlerState = Uint8Array.of()
    this.checkpoint = checkpoint
    this.subGames = []
    this.initData = gameAccount.data
    this.maxPlayers = gameAccount.maxPlayers
    this.players = players
    this.entryType = gameAccount.entryType
    this.stateSha = ''
  }

  subContext(subGame: SubGame): GameContext {
    const c = rfdc({ proto: true })(this)
    Object.setPrototypeOf(c, GameContext.prototype)
    // Use the versions from checkpoint.
    c.accessVersion = this.checkpoint.accessVersion
    c.settleVersion = this.checkpoint.getVersion(subGame.gameId)
    c.gameAddr = c.gameAddr + subGame.gameId
    c.gameId = subGame.gameId
    c.dispatch = undefined
    c.timestamp = 0n
    c.randomStates = []
    c.decisionStates = []
    c.handlerState = Uint8Array.of()
    c.checkpoint = this.checkpoint.clone()
    c.subGames = []
    c.initData = subGame.initAccount.data
    c.maxPlayers = subGame.initAccount.maxPlayers
    c.entryType = new EntryTypeDisabled({})
    c.players = []
    return c
  }

  checkpointVersion(): bigint {
    return this.checkpoint.getVersion(this.gameId)
  }

  initAccount(): InitAccount {
    const checkpoint = this.checkpoint.getData(this.gameId)
    return new InitAccount({
      maxPlayers: this.maxPlayers,
      data: this.initData,
      checkpoint,
    })
  }

  // get checkpointStateSha(): string {
  //   return this.checkpoint.getSha(this.gameId) || '';
  // }

  idToAddrUnchecked(id: bigint): string | undefined {
    return this.nodes.find(x => x.id === id)?.addr
  }

  idToAddr(id: bigint): string {
    let found = this.idToAddrUnchecked(id)
    if (found === undefined) {
      throw new Error(`Cannot map id to address: ${id.toString()}`)
    }
    return found
  }

  addrToIdUnchecked(addr: string): bigint | undefined {
    return this.nodes.find(x => x.addr === addr)?.id
  }

  addrToId(addr: string): bigint {
    let found = this.addrToIdUnchecked(addr)
    if (found === undefined) {
      throw new Error(`Cannot map address to id: ${addr}`)
    }
    return found
  }

  getNodeByAddress(addr: string): INode | undefined {
    return this.nodes.find(n => n.addr === addr)
  }

  dispatchEvent(event: GameEvent, timeout: bigint) {
    this.dispatch = {
      event,
      timeout: this.timestamp + timeout,
    }
  }

  dispatchEventInstantly(event: GameEvent) {
    this.dispatchEvent(event, 0n)
  }

  waitTimeout(timeout: bigint) {
    this.dispatch = {
      event: new WaitingTimeout({}),
      timeout: this.timestamp + timeout,
    }
  }

  actionTimeout(playerId: bigint, timeout: bigint) {
    this.dispatch = {
      event: new ActionTimeout({ playerId }),
      timeout: this.timestamp + timeout,
    }
  }

  genStartGameEvent(): GameEvent {
    return new GameStart({ accessVersion: this.accessVersion })
  }

  startGame() {
    this.dispatch = {
      event: this.genStartGameEvent(),
      timeout: 0n,
    }
  }

  shutdownGame() {
    this.dispatch = {
      event: new Shutdown({}),
      timeout: 0n,
    }
  }

  getRandomState(randomId: Id): RandomState {
    if (randomId <= 0) {
      throw new Error('Invalid random id: ' + randomId)
    }
    const st = this.randomStates[randomId - 1]
    if (st === undefined) {
      throw new Error('Invalid random id: ' + randomId)
    }
    return st
  }

  getDecisionState(decisionId: Id): DecisionState {
    if (decisionId <= 0) {
      throw new Error('Invalid decision id: ' + decisionId)
    }
    const st = this.decisionStates[decisionId - 1]
    if (st === undefined) {
      throw new Error('Invalid decision id: ' + decisionId)
    }
    return st
  }

  assign(randomId: Id, playerAddr: string, indexes: number[]) {
    const st = this.getRandomState(randomId)
    st.assign(playerAddr, indexes)
  }

  reveal(randomId: Id, indexes: number[]) {
    const st = this.getRandomState(randomId)
    st.reveal(indexes)
  }

  isRandomReady(randomId: Id): boolean {
    const k = this.getRandomState(randomId).status.kind
    return k === 'ready' || k === 'waiting-secrets'
  }

  isAllRandomReady(): boolean {
    for (const st of this.randomStates) {
      const k = st.status.kind
      if (k !== 'ready' && k !== 'waiting-secrets') {
        return false
      }
    }
    return true
  }

  isSecretsReady(): boolean {
    return this.randomStates.every(st => st.status.kind === 'ready')
  }

  setNodeStatus(addr: string, status: NodeStatus) {
    let n = this.nodes.find(n => n.addr === addr)
    if (n === undefined) {
      throw new Error('Invalid node address')
    }
    n.status = status
  }

  addNode(nodeAddr: string, accessVersion: bigint, mode: ClientMode) {
    this.nodes = this.nodes.filter(n => n.addr !== nodeAddr)
    this.nodes.push({
      addr: nodeAddr,
      id: accessVersion,
      mode,
      status: { kind: 'pending', accessVersion },
    })
  }

  setAccessVersion(accessVersion: bigint) {
    this.accessVersion = accessVersion
  }

  initRandomState(spec: RandomSpec): Id {
    const randomId = this.randomStates.length + 1
    const owners = this.nodes.filter(n => n.status.kind === 'ready' && n.mode !== 'player').map(n => n.addr)
    const randomState = new RandomState(randomId, spec, owners)
    this.randomStates.push(randomState)
    return randomId
  }

  addSharedSecrets(_addr: string, shares: SecretShare[]) {
    for (const share of shares) {
      if (share instanceof Random) {
        const { randomId, toAddr, fromAddr, index, secret } = share
        this.getRandomState(randomId).addSecret(fromAddr, toAddr, index, secret)
      } else if (share instanceof Answer) {
        const { fromAddr, decisionId, secret } = share
        this.getDecisionState(decisionId).addSecret(fromAddr, secret)
      }
    }
  }

  randomizeAndMask(addr: string, randomId: Id, ciphertexts: Ciphertext[]) {
    let st = this.getRandomState(randomId)
    st.mask(addr, ciphertexts)
    this.dispatchRandomizationTimeout(randomId)
  }

  lock(addr: string, randomId: Id, ciphertextsAndTests: CiphertextAndDigest[]) {
    let st = this.getRandomState(randomId)
    st.lock(addr, ciphertextsAndTests)
    this.dispatchRandomizationTimeout(randomId)
  }

  dispatchRandomizationTimeout(randomId: Id) {
    const noDispatch = this.dispatch === undefined
    let st = this.getRandomState(randomId)
    const statusKind = st.status.kind
    if (statusKind === 'ready') {
      this.dispatchEventInstantly(new RandomnessReady({ randomId }))
    } else if (statusKind === 'locking' || statusKind === 'masking') {
      const addr = st.status.addr
      const id = this.addrToId(addr)
      if (noDispatch) {
        this.dispatchEvent(new OperationTimeout({ ids: [id] }), OPERATION_TIMEOUT)
      }
    } else if (statusKind === 'waiting-secrets') {
      if (noDispatch) {
        const ids = st.listOperatingAddrs().map(x => this.addrToId(x))
        this.dispatchEvent(new OperationTimeout({ ids }), OPERATION_TIMEOUT)
      }
    }
  }

  bumpSettleVersion() {
    this.settleVersion += 1n
  }

  addRevealedRandom(randomId: Id, revealed: Map<number, string>) {
    const st = this.getRandomState(randomId)
    st.addRevealed(revealed)
  }

  addRevealedAnswer(decisionId: Id, revealed: string) {
    const st = this.getDecisionState(decisionId)
    st.addReleased(revealed)
  }

  ask(owner: string): Id {
    const id = this.decisionStates.length + 1
    const st = new DecisionState(id, owner)
    this.decisionStates.push(st)
    return id
  }

  answerDecision(id: Id, owner: string, ciphertext: Ciphertext, digest: Digest) {
    const st = this.getDecisionState(id)
    st.setAnswer(owner, ciphertext, digest)
  }

  getRevealed(randomId: Id): Map<number, string> {
    let st = this.getRandomState(randomId)
    return st.revealed
  }

  async applyEffect(effect: Effect): Promise<EventEffects> {
    if (effect.startGame) {
      this.startGame()
    } else if (effect.stopGame) {
      this.shutdownGame()
    } else if (effect.actionTimeout !== undefined) {
      this.actionTimeout(effect.actionTimeout.playerId, effect.actionTimeout.timeout)
    } else if (effect.waitTimeout !== undefined) {
      this.waitTimeout(effect.waitTimeout)
    } else if (effect.cancelDispatch) {
      this.dispatch = undefined
    }
    for (const assign of effect.assigns) {
      const addr = this.idToAddr(assign.playerId)
      this.assign(assign.randomId, addr, assign.indexes)
    }
    for (const reveal of effect.reveals) {
      this.reveal(reveal.randomId, reveal.indexes)
    }
    for (const ask of effect.asks) {
      this.ask(ask.playerAddr)
    }
    for (const spec of effect.initRandomStates) {
      this.initRandomState(spec)
    }

    let settles: Settle[] = []

    if (effect.handlerState !== undefined) {
      await this.setHandlerState(effect.handlerState)
      if (effect.isCheckpoint) {
        this.randomStates = []
        this.decisionStates = []
        this.bumpSettleVersion()
        this.checkpoint.setData(this.gameId, effect.handlerState)
        this.checkpoint.setAccessVersion(this.accessVersion)

        // Reset random states
        this.randomStates = []
        this.decisionStates = []

        // Sort settles and track player states
        settles.push(...effect.settles)
        settles = effect.settles
      }
    }

    for (const subGame of effect.launchSubGames) {
      this.addSubGame(subGame)
    }

    return {
      checkpoint: effect.isCheckpoint ? effect.handlerState : undefined,
      settles,
      transfers: effect.transfers,
      startGame: effect.startGame,
      launchSubGames: effect.launchSubGames,
      bridgeEvents: effect.bridgeEvents,
    }
  }

  setNodeReady(accessVersion: bigint) {
    for (const n of this.nodes) {
      if (n.status.kind === 'pending') {
        if (n.status.accessVersion <= accessVersion) {
          console.debug(`Set node ${n.addr} status to ready`)
          n.status = { kind: 'ready' }
        }
      }
    }
  }

  applyCheckpoint(accessVersion: bigint, settleVersion: bigint) {
    console.info(`Apply checkpoint, accessVersion: ${accessVersion}`)
    if (this.settleVersion !== settleVersion) {
      throw new Error(
        `Invalid checkpoint, local settle version: ${this.settleVersion}, remote settle version: ${settleVersion}`
      )
    }
    this.accessVersion = accessVersion
  }

  setTimestamp(timestamp: bigint) {
    this.timestamp = timestamp
  }

  findSubGame(gameId: number): SubGame | undefined {
    return this.subGames.find(g => g.gameId === Number(gameId))
  }

  addSubGame(subGame: SubGame) {
    const found = this.subGames.find(s => s.gameId === subGame.gameId)
    if (found === undefined) {
      this.subGames.push(subGame)
    } else {
      found.initAccount = subGame.initAccount
    }
  }

  async setHandlerState(state: Uint8Array) {
    this.stateSha = await sha256String(state)
    this.handlerState = state
  }

  addPlayer(player: ContextPlayer) {
    this.players.push(player)
  }

  removePlayer(playerId: bigint) {
    this.players = this.players.filter(p => p.id !== playerId)
  }

  getStateSha(): string {
    return this.stateSha
  }
}
