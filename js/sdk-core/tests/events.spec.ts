import { makeCustomEvent, GameEvent, ICustomEvent, Custom, serializeEvent, deserializeEvent, Ready, SecretsReady, Shutdown } from '../src/events';
import { assert } from 'chai';
import { Buffer } from 'buffer';
import * as borsh from 'borsh';

class TestCustom implements ICustomEvent {
  n: number;
  constructor(fields: { n: number }) {
    this.n = fields.n;
  }
  serialize(): Uint8Array {
    return borsh.serialize(TestCustom.schema, this);
  }
  deserialize(data: Uint8Array): ICustomEvent {
    return borsh.deserialize(TestCustom.schema, TestCustom, Buffer.from(data))
  }
  static get schema(): Map<Function, any> {
    return new Map([
      [TestCustom, {
        kind: 'struct',
        fields: [
          ['n', 'u32'],
        ]
      }]
    ])
  }
}

describe('Serialization', () => {
  it('Custom', () => {
    let e = makeCustomEvent("alice", new TestCustom({ n: 100 }))
    let data = serializeEvent(e);
    let e1 = deserializeEvent(data);
    assert.deepStrictEqual(e, e1);
  })

  it('Ready', () => {
    let e = Ready.default();
    let data = serializeEvent(e);
    let e1 = deserializeEvent(data);
    assert.deepStrictEqual(e, e1);
  })

  it('SecretsReady', () => {
    let e = SecretsReady.default();
    let data = serializeEvent(e);
    let e1 = deserializeEvent(data);
    assert.deepStrictEqual(e, e1);
  })

  it('Shutdown', () => {
    let e = Shutdown.default();
    let data = serializeEvent(e);
    let e1 = deserializeEvent(data);
    assert.deepStrictEqual(e, e1);
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
