import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import { GameBundle } from '@race-foundation/sdk-core'

const GameBundleSchema = bcs.struct('GameBundle', {
    addr: Address,              // game nft object id
    name: bcs.string(),         // game name
    symbol: bcs.string(),       // symbol (USDC, RRR, etc)
    uri: bcs.string(),          // bundle url
    cover: bcs.string()         // cover image url
})

export const GameBundleParser: Parser<GameBundle, typeof GameBundleSchema> = {
    schema: GameBundleSchema,
    transform: (input: typeof GameBundleSchema.$inferType): GameBundle => {
        return {
            addr: input.addr,
            uri: input.uri,
            name: input.name,
            data: new Uint8Array(),
        }
    }
}
