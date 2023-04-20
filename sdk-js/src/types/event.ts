import { Address, Ciphertext, RandomId, SecretDigest, SecretKey } from './common'

export type Event =
  {
    type: 'Custom'
    sender: Address
    raw: string
  }
  | {
    type: 'Ready'
    sender: Address
  }
  | {
    type: 'ShareSecrets'
    sender: Address
    secrets: Record<string, SecretKey>
  }
  | {
    type: 'Mask'
    randomId: RandomId
    ciphertexts: Ciphertext[]
  }
  | {
    type: 'Lock'
    sender: Address
    randomId: RandomId
    ciphertextsAndDigests: Array<[Ciphertext, SecretDigest]>
  }
  | {
    type: 'RandomnessReady'
  }
  | {
    type: 'Join'
    playerAddr: Address
    balance: bigint
    position: number
  }
  | {
    type: 'Leave'
    playerAddr: Address
  }
  | {
    type: 'GameStart'
  }
  | {
    type: 'WaitTimeout'
  }
  | {
    type: 'DrawRandomItems'
    randomId: RandomId
    indexes: number[]
  }
  | {
    type: 'DrawTimeout'
  }
  | {
    type: 'ActionTimeout'
    playerAddr: Address
  }
  | {
    type: 'SecretsReady'
  }

export interface CustomEvent { }

export function createCustomEvent(sender: Address, event: CustomEvent): Event {
  return {
    type: 'Custom',
    sender,
    raw: JSON.stringify(event)
  }
}
