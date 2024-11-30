import { Chain } from './common'

export interface IStorage {
    getItem(key: string): string | null
    setItem(key: string, value: any): void
}

export class TemporaryStorage {
    data: Map<string, string>
    constructor() {
        this.data = new Map()
    }
    getItem(key: string): string | null {
        const x = this.data.get(key)
        if (x === undefined) return null
        return x
    }
    setItem(key: string, value: string) {
        this.data.set(key, value)
    }
}

export type TtlCache<T> = {
    expire: number
    value: T
}

/**
 * Set a cache with a `key`, and expire after `ttl` miliseconds.
 */
export function setTtlCache(storage: IStorage, key: string, value: any, ttl: number) {
    const data = {
        expire: new Date().getTime() + ttl,
        value,
    }

    storage.setItem(key, JSON.stringify(data))
}

/**
 * Get a TTL cache value by `key`.
 */
export function getTtlCache<T>(storage: IStorage, key: string): T | undefined {
    const s = storage.getItem(key)
    if (!!s) {
        const data: TtlCache<T> = JSON.parse(s)
        if (data.expire < new Date().getTime()) {
            return undefined
        }
        return data.value
    }
    return undefined
}

export function makeBundleCacheKey(chain: Chain, bundleAddr: string): string {
    return `BUNDLE__${chain}_${bundleAddr}`
}

export function makeTokenCacheKey(chain: Chain, tokenAddr: string): string {
    return `TOKEN__${chain}_${tokenAddr}`
}

export function makeNftCacheKey(chain: Chain, nftAddr: string): string {
    return `NFT__${chain}_${nftAddr}`
}

export function makeGameAccountCacheKey(chain: Chain, gameAccountAddr: string): string {
    return `GAME_ACCOUNT_${chain}_${gameAccountAddr}`
}
