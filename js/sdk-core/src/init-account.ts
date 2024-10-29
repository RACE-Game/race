import { array, deserialize, enums, field, option, serialize, struct } from "@race-foundation/borsh";
import { EntryType, GameAccount } from "./accounts";
import { Fields } from "./types";

export class GamePlayer {
  @field('u64')
  id!: bigint;
  @field('u16')
  position!: number;
  @field('u64')
  balance!: bigint;
  constructor(fields: Fields<GamePlayer>) {
    Object.assign(this, fields)
  }
}

/**
 * A subset of GameAccount, used in handler initialization.
 */
export interface IInitAccount {
  maxPlayers: number;
  entryType: EntryType;
  players: GamePlayer[];
  data: Uint8Array;
  checkpoint: Uint8Array | undefined;
}

export class InitAccount {
  @field('u16')
  readonly maxPlayers: number;
  @field(enums(EntryType))
  readonly entryType: EntryType;
  @field(array(struct(GamePlayer)))
  readonly players: GamePlayer[];
  @field('u8-array')
  readonly data: Uint8Array;
  @field(option('u8-array'))
  readonly checkpoint: Uint8Array | undefined;

  constructor(fields: IInitAccount) {
    this.maxPlayers = fields.maxPlayers;
    this.entryType = fields.entryType;
    this.players = fields.players;
    this.data = fields.data;
    this.checkpoint = fields.checkpoint;
  }

  serialize(): Uint8Array {
    return serialize(InitAccount);
  }
  static deserialize(data: Uint8Array) {
    return deserialize(InitAccount, data);
  }
}
