import { Chain } from './types';

export let CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION',
  'solana-local': '<INVALID>',
  'solana-devnet': '',
  'solana-mainnet': 'GbnAz9VFaJ4C9jp5FCFcc9Wqzcta6vKyiDGqcz9qCQb9',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'http://localhost:12002',
  'solana-local': 'http://localhost:8899',
  'solana-devnet': 'http://api.devnet.solana.com',
  'solana-mainnet': 'https://quiet-icy-shape.solana-mainnet.quiknode.pro/8afde90d81afb1404027aaad9bbc27886cdba0d4/',
};

export let CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'facade': {
    'target>race_example_raffle>wasm': 'raffle',
  },
  'solana-local': {
    '<INVALID>': 'raffle'
  },
  'solana-devnet': {
    '': 'raffle'
  },
  'solana-mainnet': {
    'AxDr2roV3847Y7c6Ag9grL8SKTtzUo9eZpa387QUD8b7': 'raffle'
  }
};

(async function() {
  try {
    let resp = await fetch('http://localhost:8000/demo-app-data.json');
    if (resp.ok) {
      let data: any = await resp.json();
      console.log("Data: ", data);
      CHAIN_TO_REG_ADDR = Object.assign(CHAIN_TO_REG_ADDR, data["CHAIN_TO_REG_ADDR"]);
      CHAIN_ADDR_GAME_MAPPING = Object.assign(CHAIN_ADDR_GAME_MAPPING, data["CHAIN_ADDR_GAME_MAPPING"]);
      console.log("App data loaded");
    }
  } catch (e) {
    console.log("Skip local environment");
  }
})();
