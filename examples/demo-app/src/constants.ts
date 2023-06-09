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
    'target_race_example_raffle_wasm': 'raffle',
    'target_race_example_draw_card_wasm': 'draw-card',
  },
  'solana-local': {
    '<INVALID>': 'raffle'
  },
  'solana-devnet': {
    '': 'raffle'
  },
  'solana-mainnet': {
    'fLS6bq5bhMnTpSzqV54GZuBdAE8exJfCGVPyV67KmQJ': 'raffle'
  }
};

(async function() {
  try {
    let resp = await fetch('http://localhost:8000/demo-app-data.json');
    if (resp.ok) {
      let data: any = await resp.json();
      CHAIN_TO_REG_ADDR = Object.assign(CHAIN_TO_REG_ADDR, data["CHAIN_TO_REG_ADDR"]);
      CHAIN_ADDR_GAME_MAPPING = Object.assign(CHAIN_ADDR_GAME_MAPPING, data["CHAIN_ADDR_GAME_MAPPING"]);
    }
  } catch (e) {
    console.log("Skip local environment");
  }
})();
