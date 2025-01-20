import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import { GameAccount } from '@race-foundation/sdk-core'

const GameAccountSchema = bcs.struct('GameAccount', {

})

export const GameAccountParser: Parser<GameAccount, typeof GameAccountSchema> = {
    schema: GameAccountSchema,
    transform: (input: typeof GameAccountSchema.$inferType) => {
        return {}
    }
}
