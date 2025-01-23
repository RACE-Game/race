import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import { ServerAccount } from '@race-foundation/sdk-core'

const ServerSchema = bcs.struct('Server', {
    id: Address,
    owner: Address,
    endpoint: bcs.string(),
})

export const ServerParser: Parser<ServerAccount, typeof ServerSchema> = {
    schema: ServerSchema,
    transform: (input: typeof ServerSchema.$inferType): ServerAccount => {
        return {
            addr: input.id,
            endpoint: input.endpoint,
        }
    },
}
