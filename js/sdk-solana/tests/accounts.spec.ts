import { assert } from 'chai';
import {
  GameReg,
  GameState,
  PlayerJoin,
  PlayerState,
  RegistryState,
  ServerJoin,
  ServerState,
  Vote,
} from '../src/accounts';
import { PublicKey } from '@solana/web3.js';
import { ACCOUNT_DATA } from './account_data';

describe('Test account data serialization', () => {
  it('PlayerState', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: '16-char_nickname',
      pfpKey: PublicKey.default,
    });
    let buf = Buffer.from([1, 16, 0, 0, 0, 49, 54, 45, 99, 104, 97, 114, 95, 110, 105, 99, 107, 110, 97, 109, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let deserialized = PlayerState.deserialize(buf);
    assert.equal(state.nick, deserialized.nick);
    assert.deepStrictEqual(state.pfpKey, deserialized.pfpKey);
    assert.equal(state.isInitialized, deserialized.isInitialized);
  })

  it('PlayerState with no pfp', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: 'Alice',
      pfpKey: undefined,
    });
    let buf = state.serialize();
    let deserialized = PlayerState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })

  it('RegistryState', () => {
    let state = new RegistryState({
      isInitialized: true,
      isPrivate: false,
      size: 100,
      ownerKey: PublicKey.unique(),
      games: [
        new GameReg({
          gameKey: PublicKey.unique(),
          title: 'Game A',
          bundleKey: PublicKey.unique(),
          regTime: 1000n
        }),
        new GameReg({
          gameKey: PublicKey.unique(),
          title: 'Game B',
          bundleKey: PublicKey.unique(),
          regTime: 2000n
        })
      ]
    });
    let buf = state.serialize();
    let deserialized = RegistryState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })

  it('GameState deserialize', () => {
    let deserialized = GameState.deserialize(Buffer.from(ACCOUNT_DATA));
  })

  it('GameState', () => {
    let state = new GameState({
      isInitialized: true,
      title: 'test game name',
      bundleKey: PublicKey.unique(),
      stakeKey: PublicKey.unique(),
      ownerKey: PublicKey.unique(),
      tokenKey: PublicKey.unique(),
      minDeposit: 100n,
      maxDeposit: 100n,
      transactorKey: PublicKey.unique(),
      accessVersion: 1n,
      settleVersion: 2n,
      maxPlayers: 10,
      players: [
        new PlayerJoin({
          key: PublicKey.unique(),
          balance: 100n,
          accessVersion: 1n,
          position: 0,
        })
      ],
      servers: [
        new ServerJoin({
          key: PublicKey.unique(),
          endpoint: 'http://foo.bar',
          accessVersion: 2n,
        })
      ],
      dataLen: 10,
      data: Uint8Array.from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
      votes: [
        new Vote({
          voterKey: PublicKey.unique(),
          voteeKey: PublicKey.unique(),
          voteType: 0,
        })
      ],
      unlockTime: undefined
    });
    let buf = state.serialize();
    let deserialized = GameState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })

  it('ServerState', () => {
    let state = new ServerState({
      isInitialized: true,
      key: PublicKey.unique(),
      ownerKey: PublicKey.unique(),
      endpoint: 'http://foo.bar',
    });
    let buf = state.serialize();
    let deserialized = ServerState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  })


})
