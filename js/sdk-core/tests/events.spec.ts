import { makeCustomEvent } from '../src/events';
import { assert } from 'chai';

describe('Create custom event', () => {
  it('Custom string event', () => {
    let e = makeCustomEvent("addr", "Increase");
    assert.deepStrictEqual(e, {
      Custom: {
        sender: 'addr',
        raw: "\"Increase\"",
      }
    });
  });

  it('Custom object event', () => {
    let e = makeCustomEvent("addr", { Increase: 1 });
    assert.deepStrictEqual(e, {
      Custom: {
        sender: 'addr',
        raw: "{\"Increase\":1}",
      }
    });
  });
});
