import { Result } from './types'

type Signature = string
type TxError = any

export type SendTransactionResult = Result<Signature, TxError>

export interface IWallet {
  isConnected: boolean

  walletAddr: string

  wallet: any
}
