import { assert } from 'chai';
import {
  GameState,
  PlayerJoin,
  PlayerState,
  ServerJoin,
  Vote,
} from '../src/accounts';
import { PublicKey } from '@solana/web3.js';
import { VoteType, voteTypes } from 'race-sdk-core';

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

  it('GameState', () => {
    let state = new GameState({
      isInitialized: true,
      title: 'test game name',
      bundleAddr: PublicKey.unique(),
      stakeAddr: PublicKey.unique(),
      ownerAddr: PublicKey.unique(),
      tokenAddr: PublicKey.unique(),
      minDeposit: 100n,
      maxDeposit: 100n,
      transactorAddr: PublicKey.unique(),
      accessVersion: 1n,
      settleVersion: 2n,
      maxPlayers: 10,
      players: [
        new PlayerJoin({
          addr: PublicKey.unique(),
          balance: 100n,
          accessVersion: 1n,
          position: 0,
        })
      ],
      servers: [
        new ServerJoin({
          addr: PublicKey.unique(),
          endpoint: 'http://foo.bar',
          accessVersion: 2n,
        })
      ],
      dataLen: 10,
      data: Uint8Array.of(0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
      votes: [
        new Vote({
          voter: PublicKey.unique(),
          votee: PublicKey.unique(),
          voteType: 0,
        })
      ],
      unlockTime: undefined
    });
    let buf = state.serialize();
    let deserialized = GameState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })
})
