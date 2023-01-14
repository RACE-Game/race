import { Address, Amount, Position, Timestamp, Version } from './common'

export interface PlayerJoin {
  addr: Address
  position: Position
  accessVersion: Version
}

export interface PlayerDeposit {
  addr: Address
  amount: Amount
  accessVersion: Version
}

export interface ServerAccount {
  addr: Address
  ownerAddr: Address
  endpoint: string
}

export interface GameAccount {
  addr: Address
  bundleAddr: Address
  settleVersion: Version
  accessVersion: Version
  players: PlayerJoin[]
  deposits: PlayerDeposit[]
  serverAddrs: Address[]
  transactorAddr: Address | null
  maxPlayers: number
  dataLen: number
  data: Uint8Array
}

export interface GameRegistration {
  addr: Address
  regTime: Timestamp
  bundleAddr: Address
}

export interface RegistrationAccount {
  addr: Address
  isPrivate: boolean
  size: number
  owner: Address | null
  games: GameRegistration[]
}

export interface GameBundle {
  addr: Address
  data: Uint8Array
}

export interface PlayerProfile {
  addr: Address
  pfp: Address
  data: Uint8Array
}
