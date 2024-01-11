import { field, array, variant, struct } from '@race-foundation/borsh';
import { PlayerJoin } from './accounts';

// type StateField<T> = Omit<Fields<T>, 'kind'>;
export abstract class TxState {}

export class ConfirmingPlayer {
  @field('u64')
  id!: bigint;
  @field('string')
  addr!: string;
  @field('u16')
  position!: number;
  @field('u64')
  balance!: bigint;

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

@variant(0)
export class PlayerConfirming extends TxState {
  @field(array(struct(ConfirmingPlayer)))
  confirmPlayers!: ConfirmingPlayer[];
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
