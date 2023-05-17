import { makeCustomEvent, ICustomEvent, Custom, Ready, SecretsReady, Shutdown, ShareSecrets, Random, Answer, GameEvent, OperationTimeout, Mask, Lock, CiphertextAndDigest, RandomnessReady, Sync, ServerLeave, Leave, WaitingTimeout, GameStart, DrawRandomItems } from '../src/events';
import { assert } from 'chai';
import { deserialize, serialize, field } from '@race/borsh';
import { ServerJoin, PlayerJoin } from '../src/accounts';

class TestCustom implements ICustomEvent {
  @field('u32')
  n: number;
  constructor(fields: { n: number }) {
    this.n = fields.n;
  }
  serialize(): Uint8Array {
    return serialize(this);
  }
  deserialize(data: Uint8Array): ICustomEvent {
    return deserialize(TestCustom, data);
  }
}

describe('Serialization', () => {
  it('Custom', () => {
    let e = makeCustomEvent("alice", new TestCustom({ n: 100 }))
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e, e1);
  })

  it('Ready', () => {
    let e = new Ready({});
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('ShareSecrets', () => {
    let e = new ShareSecrets({
      sender: 'alice',
      shares: [
        new Random({
          fromAddr: 'alice',
          toAddr: 'bob',
          randomId: 1n,
          index: 0,
          secret: Uint8Array.of(1, 2, 3, 4),
        }),
        new Answer({
          fromAddr: 'alice',
          decisionId: 2n,
          secret: Uint8Array.of(5, 6, 7, 8),
        })
      ]
    });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('OperationTimeout', () => {
    let e = new OperationTimeout({ addrs: ['alice', 'bob'] });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('Mask', () => {
    let e = new Mask({ sender: 'alice', randomId: 1n, ciphertexts: [Uint8Array.of(1, 2, 3)] });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('Lock', () => {
    let e = new Lock({
      sender: 'alice', randomId: 1n,
      ciphertextsAndDigests: [
        new CiphertextAndDigest({
          ciphertext: Uint8Array.of(1, 2, 3),
          digest: Uint8Array.of(4, 5, 6)
        })
      ]
    });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('RandomnessReady', () => {
    let e = new RandomnessReady({
      randomId: 1n
    });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('Sync', () => {
    let e = new Sync({
      newPlayers: [
        new PlayerJoin({
          addr: 'alice',
          position: 1,
          balance: 100n,
          accessVersion: 1n
        })
      ],
      newServers: [
        new ServerJoin({
          addr: 'foo',
          endpoint: 'http://foo.bar',
          accessVersion: 2n
        })
      ],
      transactorAddr: 'foo',
      accessVersion: 2n,
    });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('ServerLeave', () => {
    let e = new ServerLeave({ serverAddr: 'foo', transactorAddr: 'bar' });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('Leave', () => {
    let e = new Leave({ playerAddr: 'foo' });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('GameStart', () => {
    let e = new GameStart({ accessVersion: 1n });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  });

  it('WaitingTimeout', () => {
    let e = new WaitingTimeout({});
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  });

  it('DrawRandomItems', () => {
    let e = new DrawRandomItems({
      sender: 'alice',
      randomId: 1n,
      indexes: [10, 20],
    });
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('SecretsReady', () => {
    let e = new SecretsReady({});
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })

  it('Shutdown', () => {
    let e = new Shutdown({});
    let data = serialize(e);
    let e1 = deserialize(GameEvent, data);
    assert.deepStrictEqual(e1, e);
  })
})

describe('Create custom event', () => {
  it('Create', () => {
    let e = makeCustomEvent("addr", new TestCustom({ n: 1 }));

    let e1 = new Custom({
      sender: 'addr',
      raw: Uint8Array.of(1, 0, 0, 0),
    });

    assert.deepStrictEqual(e, e1);
  });
});
