import { serialize, deserialize } from '@race-foundation/borsh';
import { assert } from 'chai';
import * as sut from '../src/accounts';

describe('Test accounts with borsh serialization', () => {
  it('ServerAccount', () => {
    let x = new sut.ServerAccount({
      addr: 'an addr',
      endpoint: 'http://foo.bar',
    });
    let data = serialize(x);
    assert.deepStrictEqual(
      data,
      Uint8Array.from([
        7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98,
        97, 114,
      ])
    );
    let x0 = deserialize(sut.ServerAccount, data);
    assert.deepStrictEqual(x, x0);
  });

  it('PlayerProfile without pfp', () => {
    let x = new sut.PlayerProfile({
      addr: 'an addr',
      nick: 'Alice',
      pfp: undefined,
    });
    let data = serialize(x);
    let x0 = deserialize(sut.PlayerProfile, data);
    assert.deepStrictEqual(x, x0);
  });

  it('PlayerProfile with pfp', () => {
    let x = new sut.PlayerProfile({
      addr: 'an addr',
      nick: 'Alice',
      pfp: 'Awesome PFP',
    });
    let data = serialize(x);
    assert.deepStrictEqual(
      data,
      Uint8Array.from([
        7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 5, 0, 0, 0, 65, 108, 105, 99, 101, 1, 11, 0, 0, 0, 65, 119, 101,
        115, 111, 109, 101, 32, 80, 70, 80,
      ])
    );
    let x0 = deserialize(sut.PlayerProfile, data);
    assert.deepStrictEqual(x, x0);
  });

  it('RegistrationAccount', () => {
    let x = new sut.RegistrationAccount({
      addr: 'an addr',
      isPrivate: true,
      size: 100,
      owner: 'another addr',
      games: [
        new sut.GameRegistration({
          title: 'Game A',
          addr: 'addr 0',
          regTime: BigInt(1000),
          bundleAddr: 'bundle 0',
        }),
        new sut.GameRegistration({
          title: 'Game B',
          addr: 'addr 1',
          regTime: BigInt(1001),
          bundleAddr: 'bundle 1',
        }),
      ],
    });
    let data = serialize(x);
    assert.deepStrictEqual(
      data,
      Uint8Array.from([
        7, 0, 0, 0, 97, 110, 32, 97, 100, 100, 114, 1, 100, 0, 1, 12, 0, 0, 0, 97, 110, 111, 116, 104, 101, 114, 32, 97,
        100, 100, 114, 2, 0, 0, 0, 6, 0, 0, 0, 71, 97, 109, 101, 32, 65, 6, 0, 0, 0, 97, 100, 100, 114, 32, 48, 232, 3,
        0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 48, 6, 0, 0, 0, 71, 97, 109, 101, 32, 66, 6, 0,
        0, 0, 97, 100, 100, 114, 32, 49, 233, 3, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 49,
      ])
    );
    let x0 = deserialize(sut.RegistrationAccount, data);
    assert.deepStrictEqual(x, x0);
  });

  it('GameBundle', () => {
    let x = new sut.GameBundle({
      uri: 'http://foo.bar',
      name: 'Awesome Game',
      data: Uint8Array.of(1, 2, 3, 4),
    });
    let data = serialize(x);
    assert.deepStrictEqual(
      data,
      Uint8Array.from([
        14, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 12, 0, 0, 0, 65, 119, 101, 115,
        111, 109, 101, 32, 71, 97, 109, 101, 4, 0, 0, 0, 1, 2, 3, 4,
      ])
    );
    let x0 = deserialize(sut.GameBundle, data);
    assert.deepStrictEqual(x, x0);
  });

  it('GameAccount', () => {
    let x = new sut.GameAccount({
      addr: 'game addr',
      title: 'awesome game title',
      bundleAddr: 'bundle addr',
      tokenAddr: 'token addr',
      ownerAddr: 'owner addr',
      settleVersion: BigInt(10),
      accessVersion: BigInt(20),
      players: [
        new sut.PlayerJoin({
          addr: 'player 0',
          balance: BigInt(3),
          position: 0,
          accessVersion: BigInt(19),
          verifyKey: 'VERIFY KEY',
        }),
        new sut.PlayerJoin({
          addr: 'player 1',
          balance: BigInt(6),
          position: 1,
          accessVersion: BigInt(21),
          verifyKey: 'VERIFY KEY',
        }),
      ],
      deposits: [
        new sut.PlayerDeposit({
          addr: 'player 0',
          amount: BigInt(9),
          settleVersion: BigInt(21),
        }),
        new sut.PlayerDeposit({
          addr: 'player 1',
          amount: BigInt(12),
          settleVersion: BigInt(21),
        }),
      ],
      servers: [
        new sut.ServerJoin({
          addr: 'server 0',
          endpoint: 'http://foo.bar',
          accessVersion: BigInt(17),
          verifyKey: 'VERIFY KEY',
        }),
        new sut.ServerJoin({
          addr: 'server 1',
          endpoint: 'http://foo.bar',
          accessVersion: BigInt(17),
          verifyKey: 'VERIFY KEY',
        }),
      ],
      transactorAddr: 'server 0',
      votes: [
        new sut.Vote({
          voter: 'server 1',
          votee: 'server 0',
          voteType: sut.VoteType.ServerVoteTransactorDropOff,
        }),
      ],
      unlockTime: undefined,
      maxPlayers: 30,
      dataLen: 10,
      data: Uint8Array.of(0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
      minDeposit: BigInt(100),
      maxDeposit: BigInt(250),
    });
    let data = serialize(x);

    assert.deepStrictEqual(
      data,
      Uint8Array.from([
        9, 0, 0, 0, 103, 97, 109, 101, 32, 97, 100, 100, 114, 18, 0, 0, 0, 97, 119, 101, 115, 111, 109, 101, 32, 103,
        97, 109, 101, 32, 116, 105, 116, 108, 101, 11, 0, 0, 0, 98, 117, 110, 100, 108, 101, 32, 97, 100, 100, 114, 10,
        0, 0, 0, 116, 111, 107, 101, 110, 32, 97, 100, 100, 114, 10, 0, 0, 0, 111, 119, 110, 101, 114, 32, 97, 100, 100,
        114, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32,
        48, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 19, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 86, 69, 82, 73, 70, 89, 32, 75, 69, 89,
        8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 49, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 10, 0,
        0, 0, 86, 69, 82, 73, 70, 89, 32, 75, 69, 89, 2, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 48, 9, 0,
        0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 112, 108, 97, 121, 101, 114, 32, 49, 12, 0, 0, 0, 0, 0,
        0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 14, 0, 0, 0, 104,
        116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 17, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 86, 69, 82, 73,
        70, 89, 32, 75, 69, 89, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 49, 14, 0, 0, 0, 104, 116, 116, 112, 58,
        47, 47, 102, 111, 111, 46, 98, 97, 114, 17, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 86, 69, 82, 73, 70, 89, 32, 75,
        69, 89, 1, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 1, 0, 0, 0, 8, 0, 0, 0, 115, 101, 114, 118, 101,
        114, 32, 49, 8, 0, 0, 0, 115, 101, 114, 118, 101, 114, 32, 48, 0, 0, 30, 0, 100, 0, 0, 0, 0, 0, 0, 0, 250, 0, 0,
        0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
      ])
    );
    let x0 = deserialize(sut.GameAccount, data);
    assert.deepStrictEqual(x, x0);
  });
});
