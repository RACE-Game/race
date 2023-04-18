import { assert } from 'chai';

// import * as intruction from '../src/instruction';
//
// import {
//   PROFILE_ACCOUNT_LEN,
//   PLAYER_PROFILE_SEED,
//   PROGRAM_ID,
// } from '../src/constants';
//
// import {
//   PlayerState,
// } from '../src/accounts'
import { Connection } from '@solana/web3.js';
import SolanaTransport from '../src/solana-transport';

describe('Test transport', () => {
  it('create player profile and get', () => {
    const transport = new SolanaTransport('http://localhost:8899');

    // console.log(`Successfully connected to Solana dev net.`);

  })

  it('create game account, get and close', () => {
  })

  it('join game', () => {
  })

  it('publish game and get', () => {
  })
})
