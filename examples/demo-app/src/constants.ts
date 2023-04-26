import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION_ADDRESS',
  'solana': 'Ha6yNYPpUxGNJZhaxht4Ma3wFkquwNyjVoEaJ3V5Cj1k',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'ws://localhost:12002',
  'solana': 'http://localhost:8899',
};

export const CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    'TpSf11nmAb9GX5hr3UnKiTrU67sKV77Ttt7AaCSfwWr': 'raffle'
  },
  'facade': {
    'CHAT_BUNDLE_ADDRESS': 'chat',
    'RAFFLE_BUNDLE_ADDRESS': 'raffle',
  }
};
