import { Nft } from './accounts'
import { NFT_CACHE_TTL } from './common'
import { getTtlCache, IStorage, makeNftCacheKey, setTtlCache } from './storage'
import { ITransport } from './transport'
import { PlayerProfileWithPfp, ProfileCallbackFunction } from './types'

type LoadingStatus = 'loading' | 'loaded' | 'failed'

export class ProfileLoader {
    transport: ITransport
    caches: Map<string, PlayerProfileWithPfp>
    loadingStatus: Map<string, LoadingStatus>
    storage?: IStorage
    shutdown: boolean
    onProfile?: ProfileCallbackFunction
    addrToId: Map<string, bigint>

    constructor(transport: ITransport, storage: IStorage | undefined, onProfile: ProfileCallbackFunction | undefined) {
        this.transport = transport
        this.storage = storage
        this.caches = new Map()
        this.loadingStatus = new Map()
        this.shutdown = false
        this.onProfile = onProfile
        this.addrToId = new Map()
    }

    async __loadProfile(playerAddr: string): Promise<PlayerProfileWithPfp | undefined> {
        const profile = await this.transport.getPlayerProfile(playerAddr)
        if (profile === undefined) {
            console.warn(`Player profile missing ${playerAddr}`)
            return undefined
        } else {
            let p
            if (profile.pfp !== undefined) {
                let pfp: Nft | undefined
                if (this.storage === undefined) {
                    pfp = await this.transport.getNft(profile.pfp)
                } else {
                    const cacheKey = makeNftCacheKey(this.transport.chain, profile.pfp)
                    pfp = getTtlCache(this.storage, cacheKey)
                    if (pfp === undefined) {
                        pfp = await this.transport.getNft(profile.pfp)
                        if (pfp !== undefined) {
                            setTtlCache(this.storage, cacheKey, pfp, NFT_CACHE_TTL)
                        }
                    }
                }
                p = { pfp, addr: profile.addr, nick: profile.nick }
            } else {
                p = { pfp: undefined, addr: profile.addr, nick: profile.nick }
            }
            console.debug(`Load profile, address = ${playerAddr}`, profile)
            return p
        }
    }

    getProfile(playerAddr: string): PlayerProfileWithPfp | undefined {
        return this.caches.get(playerAddr)
    }

    async start() {
        while (true) {
            if (this.shutdown) {
                break
            }
            for (const [addr, s] of this.loadingStatus) {
                if (s === 'loading') {
                    const p = await this.__loadProfile(addr)
                    if (p === undefined) {
                        this.loadingStatus.set(addr, 'failed')
                    } else {
                        if (this.onProfile !== undefined) {
                            const id = this.addrToId.get(p.addr)
                            if (id === undefined) {
                                console.warn(
                                    `Cannot find the mapping from address = ${p.addr} to id, available mapping:`,
                                    this.addrToId
                                )
                                throw new Error('Cannot find the mapping from address to id')
                            }
                            this.onProfile(id, p)
                        }
                        this.caches.set(addr, p)
                        this.loadingStatus.set(addr, 'loaded')
                    }
                }
            }
            await new Promise(r => setTimeout(r, 1000))
        }
    }

    load(id: bigint, addr: string) {
        const status = this.loadingStatus.get(addr)
        this.addrToId.set(addr, id)
        if (status === undefined) {
            console.debug(`Load profile: ${addr}, this is the first loading for this address`)
            this.loadingStatus.set(addr, 'loading')
        } else if (status === 'failed') {
            console.debug(`Load profile: ${addr}, this is a reloading after a failure`)
            this.loadingStatus.set(addr, 'loading')
        } else if (status === 'loaded') {
            console.debug(`Load profile: ${addr}, get the result from cache`)
            const p = this.caches.get(addr)
            if (p !== undefined && this.onProfile !== undefined) {
                this.onProfile(id, p)
            } else {
                console.error(`Unexpected profile cache not found, address: ${addr}`)
            }
        }
    }

    stop() {
        this.shutdown = true
    }
}
