import { GameAccount } from './accounts'

/**
 * We use this structure to cache some useful data from the fetched
 * game accounts. So we don't have to query them again. This cache
 * should be updated everytime we call [AppHelper.listTokens].
 */
export interface GameAccountCache {
    addr: string
    bundleAddr: string
    tokenAddr: string
    ownerAddr: string
    accessVersion: number
    settleVersion: number
    transactorAddr: string | undefined
    transactorEndpoint: string | undefined
}

export function makeGameAccountCache(gameAccount: GameAccount): GameAccountCache {
    return {
        addr: gameAccount.addr,
        bundleAddr: gameAccount.bundleAddr,
        accessVersion: Number(gameAccount.accessVersion),
        settleVersion: Number(gameAccount.settleVersion),
        tokenAddr: gameAccount.tokenAddr,
        ownerAddr: gameAccount.ownerAddr,
        transactorAddr: gameAccount.transactorAddr,
        transactorEndpoint: gameAccount.servers[0]?.endpoint,
    }
}
