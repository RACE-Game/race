import { assert } from 'chai';
import {
  PlayerState,
} from '../src/accounts';
import { PublicKey } from '@solana/web3.js';

describe('Test account data serialization', () => {
  it('PlayerState', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: 'Alice',
      pfp: PublicKey.default,
    });
    let buf = state.serialize();
    let deserialized = PlayerState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })

  it('PlayerState with no pfp', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: 'Alice',
      pfp: undefined,
    });
    let buf = state.serialize();
    let deserialized = PlayerState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })
})
