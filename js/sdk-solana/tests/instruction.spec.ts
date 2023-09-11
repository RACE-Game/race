import { assert } from 'chai';
import { CloseGameAccountData, CreateGameAccountData, CreatePlayerProfileData, JoinGameData } from '../src/instruction';
import { EntryTypeCash } from '@race-foundation/sdk-core';

describe('Test instruction serialization', () => {
  it('CreatePlayerProfile', () => {
    const data = new CreatePlayerProfileData('Alice');
    const serialized = data.serialize();
    const expected = Buffer.from([3, 5, 0, 0, 0, 65, 108, 105, 99, 101]);
    assert.deepStrictEqual(serialized, expected);
  });

  it('CreateGameAccount without data', () => {
    const data = new CreateGameAccountData({
      title: 'test game',
      maxPlayers: 10,
      entryType: new EntryTypeCash({
        minDeposit: BigInt(30),
        maxDeposit: BigInt(60),
      }),
      data: Uint8Array.from([]),
    });
    const serialized = data.serialize();
    const expected = Buffer.from([
      0, 9, 0, 0, 0, 116, 101, 115, 116, 32, 103, 97, 109, 101, 10, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0,
    ]);
    assert.deepStrictEqual(serialized, expected);
  });

  it('CreateGameAccount without data', () => {
    const data = new CreateGameAccountData({
      title: 'test game #2',
      entryType: new EntryTypeCash({
        minDeposit: BigInt(10),
        maxDeposit: BigInt(20),
      }),
      maxPlayers: 10,
      data: Uint8Array.of(1, 2, 3, 4),
    });
    const serialized = data.serialize();
    const expected = Buffer.from([
      0, 12, 0, 0, 0, 116, 101, 115, 116, 32, 103, 97, 109, 101, 32, 35, 50, 10, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0,
      0, 0, 0, 0, 0, 4, 0, 0, 0, 1, 2, 3, 4,
    ]);
    assert.deepStrictEqual(serialized, expected);
  });

  it('CloseGameAccount', () => {
    const data = new CloseGameAccountData();
    const serialized = data.serialize();
    const expected = Buffer.from([1]);
    assert.deepStrictEqual(serialized, expected);
  });

  it('JoinGame', () => {
    const data = new JoinGameData(1000n, 0n, 2, 'key0');
    const serialized = data.serialize();
    const expected = Buffer.from([
      10, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 4, 0, 0, 0, 107, 101, 121, 48,
    ]);
    assert.deepStrictEqual(serialized, expected);
  });
});
