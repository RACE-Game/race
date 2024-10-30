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
import { REG_ACCOUNT_DATA } from './account_data';
import { EntryTypeCash } from '@race-foundation/sdk-core';

describe('Test account data serialization', () => {
  it('PlayerState', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: '16-char_nickname',
      pfpKey: PublicKey.default,
    });
    let buf = Buffer.from([
      1, 16, 0, 0, 0, 49, 54, 45, 99, 104, 97, 114, 95, 110, 105, 99, 107, 110, 97, 109, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let deserialized = PlayerState.deserialize(buf);
    assert.equal(state.nick, deserialized.nick);
    assert.deepStrictEqual(state.pfpKey, deserialized.pfpKey);
    assert.equal(state.isInitialized, deserialized.isInitialized);
  });

  it('PlayerState with no pfp', () => {
    let state = new PlayerState({
      isInitialized: true,
      nick: 'Alice',
      pfpKey: undefined,
    });
    let buf = state.serialize();
    let deserialized = PlayerState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  });

  it('RegState deserialize', () => {
    let deserialized = RegistryState.deserialize(Buffer.from(REG_ACCOUNT_DATA));
    assert.equal(100, deserialized.size);
    assert.equal(false, deserialized.isPrivate);
    assert.equal(1, deserialized.games.length);
  });

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
          regTime: BigInt(1000),
        }),
        new GameReg({
          gameKey: PublicKey.unique(),
          title: 'Game B',
          bundleKey: PublicKey.unique(),
          regTime: BigInt(2000),
        }),
      ],
    });
    let buf = state.serialize();
    let deserialized = RegistryState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  });

  // TODO: Fix this
  // it('GameState deserialize', () => {
  //   let deserialized = GameState.deserialize(Buffer.from(ACCOUNT_DATA));
  // });

  it('GameState', () => {
    let state = new GameState({
      isInitialized: true,
      version: "0.2.2",
      title: 'test game name',
      bundleKey: PublicKey.unique(),
      stakeKey: PublicKey.unique(),
      ownerKey: PublicKey.unique(),
      tokenKey: PublicKey.unique(),
      transactorKey: PublicKey.unique(),
      accessVersion: BigInt(1),
      settleVersion: BigInt(2),
      maxPlayers: 10,
      players: [
        new PlayerJoin({
          key: PublicKey.unique(),
          balance: BigInt(100),
          accessVersion: BigInt(1),
          position: 0,
          verifyKey: 'key0',
        }),
      ],
      servers: [
        new ServerJoin({
          key: PublicKey.unique(),
          endpoint: 'http://foo.bar',
          accessVersion: BigInt(2),
          verifyKey: 'key1',
        }),
      ],
      dataLen: 10,
      data: Uint8Array.from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
      votes: [
        new Vote({
          voterKey: PublicKey.unique(),
          voteeKey: PublicKey.unique(),
          voteType: 0,
        }),
      ],
      unlockTime: undefined,
      entryType: new EntryTypeCash({
        minDeposit: BigInt(100),
        maxDeposit: BigInt(100),
      }),
      recipientAddr: PublicKey.unique(),
      checkpoint: Uint8Array.of(1, 2, 3, 4),
    });
    let buf = state.serialize();
    let deserialized = GameState.deserialize(buf);
    assert.deepStrictEqual(state, deserialized);
  });

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
  });
});
