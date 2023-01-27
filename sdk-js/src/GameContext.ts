import { RandomState } from './RandomState'
import { GameAccount } from './types/accounts'
import { Address, Amount, Position, RandomId, Settle, Timestamp } from './types/common'
import { Event } from './types/event'

export type PlayerStatus = 'absent' | 'ready' | 'disconnected' | 'drop-off'
export type ServerStatus = 'absent' | 'ready' | 'drop-off'
export type GameStatus = 'uninit' | 'initalizing' | 'waiting' | 'running' | 'sharing' | 'closed'

export interface Player {
  addr: Address
  position: Position
  status: PlayerStatus
  balance: Amount
}

export interface Server {
  addr: Address
  status: ServerStatus
}

export interface DispatchEvent {
  timeout: bigint
  event: Event
}

export class GameContext {
  readonly gameAddr: Address
  readonly transactorAddr: Address
  status: GameStatus
  players: Player[]
  servers: Server[]
  stateJson: string
  timestamp: Timestamp
  allowLeave: boolean
  randomStates: RandomState[]
  settles: Settle[] | null
  _dispatch: DispatchEvent | null

  constructor (gameAccount: GameAccount) {
    if (!gameAccount.transactorAddr) {
      throw new Error('Game is not served')
    }

    this.gameAddr = gameAccount.addr
    this.transactorAddr = gameAccount.transactorAddr
    this.status = 'uninit'
    this.players = []
    this.servers = gameAccount.serverAddrs.map(addr => ({ addr, status: 'absent' }))
    this._dispatch = null
    this.stateJson = ''
    this.timestamp = 0n
    this.allowLeave = false,
    this.randomStates = []
    this.settles = null
  }

  public get dispatch () {
    return this._dispatch
  }

  public dispatchEvent (event: Event, timeout: bigint) {
    this._dispatch = { event, timeout }
  }

  public dispatchCustomEvent (custom: CustomEvent, timeout: bigint) {
    const event: Event = {
      type: 'Custom',
      raw: JSON.stringify(custom),
      sender: this.transactorAddr
    }
    this.dispatchEvent(event, timeout)
  }

  public getRandomState (randomId: RandomId): RandomState {
    const state = this.randomStates[randomId]
    if (state == null) {
      throw new Error('Invalid random id')
    }
    return state
  }

  public assign (randomId: RandomId, playerAddr: Address, indexes: number[]) {
    const state = this.getRandomState(randomId)
    state.assign(playerAddr, indexes)
  }

  public reveal (randomId: RandomId, indexes: number[]) {
    const state = this.getRandomState(randomId)
    state.reveal(indexes)
  }

  public isAllRandomReady (): boolean {
    return this.randomStates.every(st => st._status.status === 'ready')
  }

  public getPlayerByAddress (addr: Address): Player | undefined {
    return this.players.find(p => p.addr === addr)
  }

  public setPlayerStatus (addr: Address, status: PlayerStatus) {
    const player = this.players.find(p => p.addr === addr)
    if (player == null) {
      throw new Error('Invalid player address')
    }
    player.status = status
  }

  public addPlayer (addr: Address, balance: Amount, position: Position) {
    if (this.getPlayerByAddress(addr) != null) {
      throw new Error('Player already joined')
    }
    this.players.push({
      addr, position, balance, status: 'ready'
    })
  }

  public removePlayer (addr: Address) {
    this.players.forEach((p, i) => {
      if (p.addr === addr) {
        this.players.splice(i, 1)
      }
    })
  }
}
