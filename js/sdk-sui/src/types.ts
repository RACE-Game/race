import { bcs, fromBase64, fromHex, toHex } from '@mysten/bcs'
import { PlayerProfile } from '@race-foundation/sdk-core'

const Address = bcs.bytes(32).transform({
    input: (val: string) => fromHex(val.replace(/^0x/, '')),
    output: val => `0x${toHex(val)}`,
})

const PlayerProfileSchema = bcs.struct('PlayerProfile', {
    id: Address,
    owner: Address,
    nick: bcs.string(),
    pfp: bcs.option(Address),
})

export function parsePlayerProfile(data: string): PlayerProfile {
    const playerProfile = PlayerProfileSchema.parse(fromBase64(data))
    return {
        addr: playerProfile.owner,
        nick: playerProfile.nick,
        pfp: playerProfile.pfp ? playerProfile.pfp : undefined,
    }
}

const CoinSchema = bcs.struct('Coin', {
    balance: bcs.string(),
    coinObjectId: bcs.string(),
    coinType: bcs.string(),
    digest: bcs.string(),
    previousTransaction: bcs.string(),
    version: bcs.string(),
})

export type Coin = typeof CoinSchema.$inferType

export function parseCoin(data: string): Coin {
    return CoinSchema.parse(fromBase64(data))
}
