import { TxState } from './tx-state';
import { PlayerJoin, ServerJoin } from './accounts';
import { array, enums, field, struct, variant } from '@race-foundation/borsh';
import { EventHistory, GameEvent } from './events';

export class Message {
  @field('string')
  sender!: string;
  @field('string')
  content!: string;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

export abstract class BroadcastFrame {}

@variant(0)
export class BroadcastFrameEvent extends BroadcastFrame {
  @field('string')
  target!: string;
  @field(enums(GameEvent))
  event!: GameEvent;
  @field('u64')
  timestamp!: bigint;
  @field('string')
  stateSha!: string;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, BroadcastFrameEvent.prototype);
  }
}

@variant(1)
export class BroadcastFrameMessage extends BroadcastFrame {
  @field('string')
  target!: string;
  @field(struct(Message))
  message!: Message;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, BroadcastFrameMessage.prototype);
  }
}

@variant(2)
export class BroadcastFrameTxState extends BroadcastFrame {
  @field(enums(TxState))
  txState!: TxState;
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, BroadcastFrameTxState.prototype);
  }
}

@variant(3)
export class BroadcastFrameSync extends BroadcastFrame {
  @field(array(struct(PlayerJoin)))
  newPlayers!: PlayerJoin[];
  @field(array(struct(ServerJoin)))
  newServers!: ServerJoin[];
  @field('string')
  transactor_addr!: string;
  @field('u64')
  accessVersion!: bigint;
  constructor(fields: any) {
    super();
    Object.assign(this, fields)
    Object.setPrototypeOf(this, BroadcastFrameSync.prototype);
  }
}

@variant(4)
export class BroadcastFrameEventHistories extends BroadcastFrame {
  @field('string')
  gameAddr!: string;
  @field(array(struct(EventHistory)))
  histories!: EventHistory[];
  constructor(fields: any) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, BroadcastFrameEventHistories.prototype);
  }
}
