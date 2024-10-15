import { array, deserialize, enums, field, serialize, struct } from "@race-foundation/borsh";
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

  constructor(fields: IInitAccount) {
    this.maxPlayers = fields.maxPlayers;
    this.entryType = fields.entryType;
    this.players = fields.players;
    this.data = fields.data;
  }

  static createFromGameAccount(
    gameAccount: GameAccount,
  ): InitAccount {
    let { players, data, checkpoint } = gameAccount;
    const game_players = players.filter(p => p.accessVersion <= checkpoint.accessVersion)
      .map(p => new GamePlayer({ id: p.accessVersion, balance: p.balance, position: p.position }));

    return new InitAccount({
      data,
      players: game_players,
      maxPlayers: gameAccount.maxPlayers,
      entryType: gameAccount.entryType,
    });
  }
  serialize(): Uint8Array {
    return serialize(InitAccount);
  }
  static deserialize(data: Uint8Array) {
    return deserialize(InitAccount, data);
  }
}
