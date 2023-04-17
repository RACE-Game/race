import { assert } from 'chai';
import { CreatePlayerProfile } from '../src/instruction';
import {
  deserialize,
  serialize,
} from "@dao-xyz/borsh";

describe('Test transport', () => {
  it('create player profile', () => {
    console.log(CreatePlayerProfile);
    const ix = new CreatePlayerProfile("Alice");
    const serialized = serialize(ix);
    const expected = Uint8Array.from([3, 5, 0, 0, 0, 65, 108, 105, 99, 101]);
    assert.equal(serialized, expected);
  })
})
