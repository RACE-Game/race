import {
  deserialize,
  serialize,
  field,
  variant,
  vec,
  option
} from "@dao-xyz/borsh";

abstract class Instruction {};

@variant(3)
export class CreatePlayerProfile extends Instruction {
  @field({ type: 'string' })
  public nick: string

  constructor(nick: string) {
    super();
    this.nick = nick;
  }

}

// export enum Instruction {
//   CreateGameAccount = 0,
//   CloseGameAccount = 1,
//   Join = 2,
//   Deposit = 3,
//   PublishGame = 4,
//   Vote = 5,
//   CreatePlayerProfile = 6,
//   CreateRegistration = 7,
//   RegisterGame = 8,
//   UnregisterGame = 9,
// };
