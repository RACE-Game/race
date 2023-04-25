import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
  'facade': 'DEFAULT_REGISTRATION_ADDRESS',
  'solana': 'DUMZU76rYgYkXuaCBB2m65USqeeCd1T1Gk2mpfgKKrS4',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
  'facade': 'ws://localhost:12002',
  'solana': 'http://localhost:8899',
};

export const CHAIN_ADDR_GAME_MAPPING: Record<Chain, Record<string, string>> = {
  'solana': {
    '96VJaPnUHuCGsvX1eACQPDU5R7WxwB2TxQY6CL947DqF': 'raffle'
  },
  'facade': {
    'CHAT_BUNDLE_ADDRESS': 'chat',
    'RAFFLE_BUNDLE_ADDRESS': 'raffle',
  }
};
