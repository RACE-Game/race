import { IWallet, SendTransactionResult } from '@race-foundation/sdk-core'
import { makeid } from './utils'

export class FacadeWallet implements IWallet {
    #addr: string

    constructor()
    constructor(addr: string)
    constructor(addr?: string) {
        if (addr === undefined) {
            this.#addr = makeid(16)
        } else {
            this.#addr = addr
        }
    }

    get isConnected() {
        return true
    }

    get walletAddr() {
        return this.#addr
    }

    get wallet() {
        return undefined
    }
}
