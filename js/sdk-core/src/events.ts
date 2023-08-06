import { field, array, enums, option, variant, struct } from '@race-foundation/borsh';
import { PlayerJoin, ServerJoin } from './accounts';
import { Fields, Id } from './types';

type EventFields<T> = Omit<Fields<T>, 'kind'>;

export type EventKind =
    | 'Invalid' // an invalid value
    | 'Custom'
    | 'Ready'
    | 'ShareSecrets'
    | 'OperationTimeout'
    | 'Mask'
    | 'Lock'
    | 'RandomnessReady'
    | 'Sync'
    | 'ServerLeave'
    | 'Leave'
    | 'GameStart'
    | 'WaitingTimeout'
    | 'DrawRandomItems'
    | 'DrawTimeout'
    | 'ActionTimeout'
    | 'AnswerDecision'
    | 'SecretsReady'
    | 'Shutdown';

export interface ICustomEvent {
    serialize(): Uint8Array;
}

interface IEventKind {
    kind(): EventKind;
}

export abstract class SecretShare {}

@variant(0)
export class Random extends SecretShare {
    @field('string')
    fromAddr!: string;
    @field(option('string'))
    toAddr!: string | undefined;
    @field('usize')
    randomId!: Id;
    @field('usize')
    index!: number;
    @field('u8-array')
    secret!: Uint8Array;
    constructor(fields: EventFields<Random>) {
        super();
        Object.assign(this, fields);
    }
}

@variant(1)
export class Answer extends SecretShare {
    @field('string')
    fromAddr!: string;
    @field('usize')
    decisionId!: Id;
    @field('u8-array')
    secret!: Uint8Array;
    constructor(fields: EventFields<Answer>) {
        super();
        Object.assign(this, fields);
    }
}

export abstract class GameEvent implements IEventKind {
    kind(): EventKind {
        return 'Invalid';
    }
}

@variant(0)
export class Custom extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field('u8-array')
    raw!: Uint8Array;
    constructor(fields: EventFields<Custom>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'Custom';
    }
}

export function makeCustomEvent(sender: string, customEvent: ICustomEvent): Custom {
    return new Custom({
        sender,
        raw: customEvent.serialize(),
    });
}

@variant(1)
export class Ready extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super();
    }
    kind(): EventKind {
        return 'Ready';
    }
}

@variant(2)
export class ShareSecrets extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field(array(enums(SecretShare)))
    shares!: SecretShare[];
    constructor(fields: EventFields<ShareSecrets>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'ShareSecrets';
    }
}

@variant(3)
export class OperationTimeout extends GameEvent implements IEventKind {
    @field(array('string'))
    addrs!: string[];
    constructor(fields: EventFields<OperationTimeout>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'OperationTimeout';
    }
}

@variant(4)
export class Mask extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field('usize')
    randomId!: Id;
    @field(array('u8-array'))
    ciphertexts!: Uint8Array[];
    constructor(fields: EventFields<Mask>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'Mask';
    }
}

export class CiphertextAndDigest {
    @field('u8-array')
    ciphertext!: Uint8Array;
    @field('u8-array')
    digest!: Uint8Array;
    constructor(fields: EventFields<CiphertextAndDigest>) {
        Object.assign(this, fields);
    }
}

@variant(5)
export class Lock extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field('usize')
    randomId!: Id;
    @field(array(struct(CiphertextAndDigest)))
    ciphertextsAndDigests!: CiphertextAndDigest[];
    constructor(fields: EventFields<Lock>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'Lock';
    }
}

@variant(6)
export class RandomnessReady extends GameEvent implements IEventKind {
    @field('usize')
    randomId!: Id;
    constructor(fields: EventFields<RandomnessReady>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'RandomnessReady';
    }
}

@variant(7)
export class Sync extends GameEvent implements IEventKind {
    @field(array(struct(PlayerJoin)))
    newPlayers!: PlayerJoin[];
    @field(array(struct(ServerJoin)))
    newServers!: ServerJoin[];
    @field('string')
    transactorAddr!: string;
    @field('u64')
    accessVersion!: bigint;
    constructor(fields: EventFields<Sync>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'Sync';
    }
}

@variant(8)
export class ServerLeave extends GameEvent implements IEventKind {
    @field('string')
    serverAddr!: string;
    @field('string')
    transactorAddr!: string;
    constructor(fields: EventFields<ServerLeave>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'ServerLeave';
    }
}

@variant(9)
export class Leave extends GameEvent implements IEventKind {
    @field('string')
    playerAddr!: string;
    constructor(fields: EventFields<Leave>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'Leave';
    }
}

@variant(10)
export class GameStart extends GameEvent implements IEventKind {
    @field('u64')
    accessVersion!: bigint;
    constructor(fields: EventFields<GameStart>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'GameStart';
    }
}

@variant(11)
export class WaitingTimeout extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super();
    }
    kind(): EventKind {
        return 'WaitingTimeout';
    }
}

@variant(12)
export class DrawRandomItems extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field('usize')
    randomId!: Id;
    @field(array('usize'))
    indexes!: number[];
    constructor(fields: EventFields<DrawRandomItems>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'DrawRandomItems';
    }
}

@variant(13)
export class DrawTimeout extends GameEvent implements IEventKind {
    constructor(_: {}) {
        super();
    }
    kind(): EventKind {
        return 'DrawTimeout';
    }
}

@variant(14)
export class ActionTimeout extends GameEvent implements IEventKind {
    @field('string')
    playerAddr!: string;
    constructor(fields: EventFields<ActionTimeout>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'ActionTimeout';
    }
}

@variant(15)
export class AnswerDecision extends GameEvent implements IEventKind {
    @field('string')
    sender!: string;
    @field('usize')
    decisionId!: Id;
    @field('u8-array')
    ciphertext!: Uint8Array;
    @field('u8-array')
    digest!: Uint8Array;
    constructor(fields: EventFields<AnswerDecision>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'AnswerDecision';
    }
}

@variant(16)
export class SecretsReady extends GameEvent implements IEventKind {
    @field(array('usize'))
    randomIds!: number[];

    constructor(fields: EventFields<SecretsReady>) {
        super();
        Object.assign(this, fields);
    }
    kind(): EventKind {
        return 'SecretsReady';
    }
}

@variant(17)
export class Shutdown extends GameEvent implements IEventKind {
    constructor(_: any = {}) {
        super();
    }
    kind(): EventKind {
        return 'Shutdown';
    }
}
