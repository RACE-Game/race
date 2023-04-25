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
import { ACCOUNT_DATA, REG_ACCOUNT_DATA } from './account_data';
import { Creator, Metadata } from '../src/metadata';

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

  it('RegState deserialize', () => {
    let deserialized = RegistryState.deserialize(Buffer.from(REG_ACCOUNT_DATA));
    assert.equal(100, deserialized.size);
    assert.equal(false, deserialized.isPrivate);
    assert.equal(1, deserialized.games.length);
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
          regTime: BigInt(1000)
        }),
        new GameReg({
          gameKey: PublicKey.unique(),
          title: 'Game B',
          bundleKey: PublicKey.unique(),
          regTime: BigInt(2000)
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
      minDeposit: BigInt(100),
      maxDeposit: BigInt(100),
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
        })
      ],
      servers: [
        new ServerJoin({
          key: PublicKey.unique(),
          endpoint: 'http://foo.bar',
          accessVersion: BigInt(2),
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

  it('Data of Metadata ', () => {
    // let data =  new Data {
    //   name: '',
    //   symbol: 'RACEBUNDLE',
    //   uri: 'https://arweave.net/hTlRRw2tZ2q_RpklixjRIzrR3PgAChBujAKzp6wknHs',
    //   sellerFeeBasisPoints: 0,
    //   creators: [
    //     {
    //       address: 'J22ir2nLxVRqUrcpwMDBM47HpFCLLRrKFroF6LjK7DEA',
    //       verfied: false,
    //       share: 100,
    //
    //     },
    //     {
    //       address: 'J22ir2nLxVRqUrcpwMDBM47HpFCLLRrKFroF6LjK7DEA',
    //       verfied: false,
    //       share: 100,
    //
    //     }
    //   ];
    // };

    let buf = [4,252,218,52,124,238,86,26,253,123,82,9,204,88,179,130,75,100,208,135,90,186,48,130,195,185,34,36,232,57,172,46,59,120,69,102,219,252,59,237,51,163,26,177,248,62,248,182,70,187,170,199,95,7,206,244,170,76,122,135,163,167,127,98,254,32,0,0,0,82,97,102,102,108,101,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,10,0,0,0,82,65,67,69,66,85,78,68,76,69,200,0,0,0,104,116,116,112,115,58,47,47,97,114,119,101,97,118,101,46,110,101,116,47,104,84,108,82,82,119,50,116,90,50,113,95,82,112,107,108,105,120,106,82,73,122,114,82,51,80,103,65,67,104,66,117,106,65,75,122,112,54,119,107,110,72,115,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,1,2,0,0,0,252,218,52,124,238,86,26,253,123,82,9,204,88,179,130,75,100,208,135,90,186,48,130,195,185,34,36,232,57,172,46,59,0,100,120,69,102,219,252,59,237,51,163,26,177,248,62,248,182,70,187,170,199,95,7,206,244,170,76,122,135,163,167,127,98,254,0,0,0,0,1,255,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

    let der = Metadata.deserialize(Buffer.from(buf));
    console.log(der)
    // assert.equal(1,2);
  })

})
