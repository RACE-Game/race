import { array, deserialize, enums, field, option, serialize, struct } from '@race-foundation/borsh'
import { EntryType, GameAccount } from './accounts'
import { Fields } from './types'

/**
 * A subset of GameAccount, used in handler initialization.
 */
export class InitAccount {
    @field('u16')
    readonly maxPlayers: number
    @field('u8-array')
    readonly data: Uint8Array

    constructor(fields: Fields<InitAccount>) {
        this.maxPlayers = fields.maxPlayers
        this.data = fields.data
    }

    serialize(): Uint8Array {
        return serialize(InitAccount)
    }

    static deserialize(data: Uint8Array) {
        return deserialize(InitAccount, data)
    }
}
