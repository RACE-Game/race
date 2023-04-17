import { assert } from 'chai';
import {
  CreatePlayerProfileData,
  createPlayerProfileDataScheme,
} from '../src/instruction';
import * as borsh from 'borsh';

describe('Test instruction serialization', () => {
  it('CreatePlayerProfile', () => {
    const data = new CreatePlayerProfileData("Alice");
    const serialized = borsh.serialize(createPlayerProfileDataScheme, data);
    const expected = Buffer.from([3, 5, 0, 0, 0, 65, 108, 105, 99, 101])
    assert.deepStrictEqual(serialized, expected);
  })
})
