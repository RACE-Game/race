import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import { PlayerProfile } from '@race-foundation/sdk-core'

const PlayerProfileSchema = bcs.struct('PlayerProfile', {
    id: Address,
    owner: Address,
    nick: bcs.string(),
    pfp: bcs.option(Address),
})

export const PlayerPorfileParser: Parser<PlayerProfile, typeof PlayerProfileSchema> = {
    schema: PlayerProfileSchema,
    transform: (input: typeof PlayerProfileSchema.$inferType): PlayerProfile => {
        return {
            addr: input.owner,
            nick: input.nick,
            pfp: input.pfp ? input.pfp : undefined,
        }
    },
}
