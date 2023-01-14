import { Address, Ciphertext, SecretDigest, SecretKey } from './types/common'

export interface RandomSpec {
  options: () => string[]
  size: () => number
}

export class ShuffledList implements RandomSpec {
  readonly _options: string[]

  constructor (options: string[]) {
    this._options = options
  }

  options (): string[] {
    return this._options
  }

  size (): number {
    return this._options.length
  }
}

export type MaskStatus = 'required' | 'applied' | 'removed'

export interface Mask {
  status: MaskStatus
  owner: Address
}

export interface Lock {
  digest: SecretDigest
  owner: Address
}

export type CipherOwner =
  { type: 'unclaimed' }
  | { type: 'assigned', addr: Address }
  | { type: 'multi-assigned', addrs: Address[] }
  | { type: 'revealed' }

export class LockedCiphertext {
  locks: Lock[]
  owner: CipherOwner
  ciphertext: Ciphertext

  constructor (ciphertext: Ciphertext) {
    this.locks = []
    this.owner = { type: 'unclaimed' }
    this.ciphertext = ciphertext
  }
}

export interface SecretShare {
  fromAddr: Address
  toAddr: Address | null
  index: number
  secret: SecretKey | null
}

export type RandomStatus =
  { status: 'ready' }
  | { status: 'locking', addr: Address }
  | { status: 'masking', addr: Address }
  | { status: 'waiting-secrets' }

export function deckOfCards (): ShuffledList {
  return new ShuffledList([
    'ha', 'h2', 'h3', 'h4', 'h5', 'h6', 'h7', 'h8', 'h9', 'ht', 'hj', 'hq', 'hk', 'sa', 's2',
    's3', 's4', 's5', 's6', 's7', 's8', 's9', 'st', 'sj', 'sq', 'sk', 'da', 'd2', 'd3', 'd4',
    'd5', 'd6', 'd7', 'd8', 'd9', 'dt', 'dj', 'dq', 'dk', 'ca', 'c2', 'c3', 'c4', 'c5', 'c6',
    'c7', 'c8', 'c9', 'ct', 'cj', 'cq', 'ck'
  ])
}
