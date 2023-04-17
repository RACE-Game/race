import { assert } from 'chai';
import { CreatePlayerProfile } from '../src/instruction';
import {
  serialize,
} from "@dao-xyz/borsh";

describe('Test instruction serialization', () => {
  it('CreatePlayerProfile', () => {
    console.log(CreatePlayerProfile);
    const ix = new CreatePlayerProfile("Alice");
    const serialized = serialize(ix);
    const expected = Buffer.from([3, 5, 0, 0, 0, 65, 108, 105, 99, 101])
    assert.deepStrictEqual(serialized, expected);
  })
})
