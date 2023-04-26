import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION_ADDRESS',
  'solana': 'B5csE2RUKp9ifBNmPmEdPdMvbazU8HK7NHsk6cDniEZ3',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'ws://localhost:12002',
  'solana': 'http://localhost:8899',
};

export const CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    '3WV4q5nDBKUuDdwStEr7STmjKaVxSdutcR9VtcTzp72r': 'raffle'
  },
  'facade': {
    'CHAT_BUNDLE_ADDRESS': 'chat',
    'RAFFLE_BUNDLE_ADDRESS': 'raffle',
  }
};
