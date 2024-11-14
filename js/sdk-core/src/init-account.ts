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
  @field(option('u8-array'))
  readonly checkpoint: Uint8Array | undefined

  constructor(fields: Fields<InitAccount>) {
    this.maxPlayers = fields.maxPlayers
    this.data = fields.data
    this.checkpoint = fields.checkpoint
  }

  serialize(): Uint8Array {
    return serialize(InitAccount)
  }
  static deserialize(data: Uint8Array) {
    return deserialize(InitAccount, data)
  }
}
