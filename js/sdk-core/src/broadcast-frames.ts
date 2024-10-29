import { TxState } from './tx-state';
import { PlayerJoin, ServerJoin } from './accounts';
import { array, enums, field, option, struct, variant } from '@race-foundation/borsh';
import { EventHistory, GameEvent } from './events';
import { CheckpointOffChain } from './checkpoint';

export type BroadcastFrameKind =
  | 'Invalid'
  | 'Event'
  | 'Message'
  | 'TxState'
  | 'Sync'
  | 'EventHistories'

export class Message {
  @field('string')
  sender!: string;
  @field('string')
  content!: string;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

export abstract class BroadcastFrame {
  kind(): BroadcastFrameKind {
    return 'Invalid'
  }
}

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
  kind(): BroadcastFrameKind {
    return 'Event'
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
  kind(): BroadcastFrameKind {
    return 'Message'
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
  kind(): BroadcastFrameKind {
    return 'TxState'
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
  kind(): BroadcastFrameKind {
    return 'Sync'
  }
}

@variant(4)
export class BroadcastFrameEventHistories extends BroadcastFrame {
  @field('string')
  gameAddr!: string;
  @field(option(struct(CheckpointOffChain)))
  checkpointOffChain: CheckpointOffChain | undefined;
  @field(array(struct(EventHistory)))
  histories!: EventHistory[];
  @field('string')
  stateSha!: string;

  constructor(fields: any) {
    super();
    Object.assign(this, fields);
    Object.setPrototypeOf(this, BroadcastFrameEventHistories.prototype);
  }
  kind(): BroadcastFrameKind {
    return 'EventHistories'
  }
}
