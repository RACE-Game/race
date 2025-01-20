import { Result } from './types'

export type SendTransactionResult<Sig> = Result<Sig, any>

export interface IWallet {
    isConnected: boolean

    walletAddr: string

    wallet: any
}
