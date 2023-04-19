import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION_ADDRESS',
  'solana': 'HeQWXkc8Bq7eDRaEhP4X73bWw9mGqYwoi7M3MxPtWWtA',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'ws://localhost:12002',
  'solana': 'http://localhost:8899',
};

export const CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    'ES6Zpewa3XBcpBGhG7NSKgqFj7Nixzdgg21ANVs7wEUY': 'raffle'
  },
  'facade': {
    'CHAT_BUNDLE_ADDRESS': 'chat',
    'RAFFLE_BUNDLE_ADDRESS': 'raffle',
  }
};
