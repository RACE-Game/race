import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import { RegistrationAccount } from '@race-foundation/sdk-core'

// Define the GameRegistrationSchema to use in RegistrationAccountSchema
const GameRegistrationSchema = bcs.struct('GameRegistration', {
    title: bcs.string(),
    addr: Address,
    regTime: bcs.u64(),
    bundleAddr: Address,
})

// Define the RegistrationAccountSchema
const RegistrationAccountSchema = bcs.struct('RegistrationAccount', {
    addr: Address,
    isPrivate: bcs.bool(),
    size: bcs.u16(),
    owner: bcs.option(Address),
    games: bcs.vector(GameRegistrationSchema),
})

// Create the parser for RegistrationAccount
export const RegistrationAccountParser: Parser<RegistrationAccount, typeof RegistrationAccountSchema> = {
    schema: RegistrationAccountSchema,
    transform: (input: typeof RegistrationAccountSchema.$inferType): RegistrationAccount => {
        return {
            addr: input.addr,
            isPrivate: input.isPrivate,
            size: input.size,
            owner: input.owner ? input.owner : undefined,
            games: input.games.map(game => ({
                title: game.title,
                addr: game.addr,
                regTime: BigInt(game.regTime),
                bundleAddr: game.bundleAddr,
            })),
        }
    },
}
