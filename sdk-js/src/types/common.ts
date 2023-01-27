export type Address = string
export type Amount = bigint
export type Position = number
export type Version = bigint
export type Ciphertext = Uint8Array
export type RandomId = number
export type SecretDigest = Uint8Array
export type SecretKey = Uint8Array
export type Timestamp = bigint

export type ClientMode = 'player' | 'transactor' | 'validator'
export type RandomMode = 'shuffler' | 'drawer'
export type Chain = 'facade' | 'solana' | 'bnb'

export interface SecretIdent {
  fromAddr: Address
  toAddr: Address | null
  randomId: RandomId
  index: number
}

export type PlayerStatus = 'normal' | 'left' | 'dropout'
export type AssetChange = 'add' | 'sub' | 'no-change'

export interface Settle {
  addr: Address
  status: PlayerStatus
  change: AssetChange
  amount: bigint
}
