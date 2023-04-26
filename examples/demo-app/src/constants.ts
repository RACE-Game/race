import { Chain } from './types';

export let CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'solana-local': '<INVALID>',
  'solana-devnet': '',
  'solana-mainnet': 'GbnAz9VFaJ4C9jp5FCFcc9Wqzcta6vKyiDGqcz9qCQb9',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'solana-local': 'http://localhost:8899',
  'solana-devnet': 'http://api.devnet.solana.com',
  'solana-mainnet': 'https://quiet-icy-shape.solana-mainnet.quiknode.pro/8afde90d81afb1404027aaad9bbc27886cdba0d4/',
};

export let CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana-local': {
    '<INVALID>': 'raffle'
  },
  'solana-devnet': {
    '': 'raffle'
  },
  'solana-mainnet': {
    '9sFs2Brfqtxwjj1BgsXe7y4yAs8T6ueLf3dCgff5JBB5': 'raffle'
  }
};

(async function() {
  try {
    let resp = await fetch('http://localhost:8000/demo-app-data.json');
    if (resp.ok) {
      let data: any = await resp.json();
      Object.assign(CHAIN_TO_REG_ADDR, data["CHAIN_TO_REG_ADDR"]);
      Object.assign(CHAIN_ADDR_GAME_MAPPING, data["CHAIN_ADDR_GAME_MAPPING"]);
      console.log("App data loaded");
    }
  } catch (e) {
    console.log("Skip local environment");
  }
})();
