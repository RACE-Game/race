import {
    makeCustomEvent,
    ICustomEvent,
    Custom,
    Ready,
    SecretsReady,
    Shutdown,
    ShareSecrets,
    Random,
    Answer,
    GameEvent,
    OperationTimeout,
    Mask,
    Lock,
    CiphertextAndDigest,
    RandomnessReady,
    Join,
    ServerLeave,
    Leave,
    WaitingTimeout,
    GameStart,
    DrawRandomItems,
    GamePlayer,
} from '../src/events'

import { assert } from 'chai'
import { deserialize, serialize, field } from '@race-foundation/borsh'

class TestCustom implements ICustomEvent {
    @field('u32')
    n: number
    constructor(fields: { n: number }) {
        this.n = fields.n
    }
    serialize(): Uint8Array {
        return serialize(this)
    }
    deserialize(data: Uint8Array): ICustomEvent {
        return deserialize(TestCustom, data)
    }
}

describe('Serialization', () => {
    it('Custom', () => {
        let e = makeCustomEvent(1n, new TestCustom({ n: 100 }))
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e, e1)
    })

    it('Ready', () => {
        let e = new Ready({})
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('ShareSecrets', () => {
        let e = new ShareSecrets({
            sender: 1n,
            shares: [
                new Random({
                    fromAddr: 'alice',
                    toAddr: 'bob',
                    randomId: 1,
                    index: 0,
                    secret: Uint8Array.of(1, 2, 3, 4),
                }),
                new Answer({
                    fromAddr: 'alice',
                    decisionId: 2,
                    secret: Uint8Array.of(5, 6, 7, 8),
                }),
            ],
        })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('OperationTimeout', () => {
        let e = new OperationTimeout({ ids: [1n, 2n] })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('Mask', () => {
        let e = new Mask({ sender: 1n, randomId: 1, ciphertexts: [Uint8Array.of(1, 2, 3)] })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('Lock', () => {
        let e = new Lock({
            sender: 1n,
            randomId: 1,
            ciphertextsAndDigests: [
                new CiphertextAndDigest({
                    ciphertext: Uint8Array.of(1, 2, 3),
                    digest: Uint8Array.of(4, 5, 6),
                }),
            ],
        })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('RandomnessReady', () => {
        let e = new RandomnessReady({
            randomId: 1,
        })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('Join', () => {
        let e = new Join({
            players: [
                new GamePlayer({
                    id: 1n,
                    position: 1,
                }),
            ],
        })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('ServerLeave', () => {
        let e = new ServerLeave({ serverId: 2n })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('Leave', () => {
        let e = new Leave({ playerId: 1n })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('GameStart', () => {
        let e = new GameStart({ accessVersion: 1n })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('WaitingTimeout', () => {
        let e = new WaitingTimeout({})
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('DrawRandomItems', () => {
        let e = new DrawRandomItems({
            sender: 1n,
            randomId: 1,
            indexes: [10, 20],
        })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('SecretsReady', () => {
        let e = new SecretsReady({ randomIds: [1, 2] })
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })

    it('Shutdown', () => {
        let e = new Shutdown({})
        let data = serialize(e)
        let e1 = deserialize(GameEvent, data)
        assert.deepStrictEqual(e1, e)
    })
})

describe('Create custom event', () => {
    it('Create', () => {
        let e = makeCustomEvent(1n, new TestCustom({ n: 1 }))

        let e1 = new Custom({
            sender: 1n,
            raw: Uint8Array.of(1, 0, 0, 0),
        })

        assert.deepStrictEqual(e, e1)
    })
})
