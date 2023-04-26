import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION_ADDRESS',
  'solana': 'E2SWM1cRm1mWUc2QAaZ9ZxQdLnTiZyEXs6CV4EaKs5jD',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'ws://localhost:12002',
  'solana': 'http://localhost:8899',
};

export const CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    'ott9NFEU1mqfowQ45RbExQBwhidsfdXb9scu9Y9mRyN': 'raffle'
  },
  'facade': {
    'CHAT_BUNDLE_ADDRESS': 'chat',
    'RAFFLE_BUNDLE_ADDRESS': 'raffle',
  }
};
