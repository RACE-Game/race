import { Chain } from './types';

export let CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'solana': '<INVALID>',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'solana': 'http://localhost:8899',
};

export let CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    '<INVALID>': 'raffle'
  },
};

(async function(){
  let resp = await fetch('http://localhost:8000/demo-app-data.json');
  let data: any = await resp.json();
  CHAIN_TO_REG_ADDR = data["CHAIN_TO_REG_ADDR"];
  CHAIN_ADDR_GAME_MAPPING = data["CHAIN_ADDR_GAME_MAPPING"];
  console.log("App data loaded");
})();
