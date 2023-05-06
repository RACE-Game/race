import { makeCustomEvent, ICustomEvent, Custom, Ready, SecretsReady, Shutdown, ShareSecrets, Random, Answer, GameEvent } from '../src/events';
import { assert } from 'chai';
import { deserialize, serialize, field } from '@race/borsh';

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
