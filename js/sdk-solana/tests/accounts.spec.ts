import { assert } from 'chai';
import {
  PlayerState,
  ExtendedReader,
  ExtendedWriter,
  playerStateSchema
} from '../src/accounts';
import * as borsh from 'borsh';
import { PublicKey } from '@solana/web3.js';

describe('Test account data serialization', () => {
  it('PlayerState', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: 'Alice',
      pfp: PublicKey.default,
    });
    let buf = borsh.serialize(playerStateSchema, state, ExtendedWriter)
    let deserialized = borsh.deserializeUnchecked(playerStateSchema, PlayerState, Buffer.from(buf), ExtendedReader);
    assert.deepStrictEqual(state, deserialized);
  })
})
