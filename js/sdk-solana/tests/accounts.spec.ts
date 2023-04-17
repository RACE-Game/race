import { assert } from 'chai';
import { PlayerState } from '../src/accounts';
import {
  deserialize,
  serialize,
} from "@dao-xyz/borsh";
import { PublicKey } from '@solana/web3.js';

describe('Test account data serialization', () => {
  it('PlayerState', () => {
    let state = new PlayerState({
      is_initialized: true,
      nick: 'Alice',
      pfp: PublicKey.default,
      padding: Uint8Array.from([0, 0, 0])
    });
    let serialized = serialize(state);
    let deserialized = deserialize(serialized, PlayerState);
    assert.deepStrictEqual(state, deserialized);
  })
})
