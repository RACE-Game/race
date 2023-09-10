import { field, array, enums, option, variant, struct } from '@race-foundation/borsh';
import { PlayerJoin } from './accounts';

// type StateField<T> = Omit<Fields<T>, 'kind'>;
export abstract class TxState {}

@variant(0)
export class PlayerConfirming extends TxState {
  @field(array(struct(PlayerJoin)))
  confirmPlayers!: PlayerJoin[];
  @field('u64')
  accessVersion!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}

@variant(1)
export class PlayerConfirmingFailed extends TxState {
  @field('u64')
  accessVersion!: bigint;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
  }
}
