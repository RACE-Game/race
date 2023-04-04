import { Chain } from './types';

export const CHAIN_TO_REG_ADDR: Record<Chain, string> = {
    'facade': 'DEFAULT_REGISTRATION_ADDRESS',
    'solana': 'aXjc4aGdRkatZNXLTDP9mZcDVpgTVmkfV8VpdgdZZuw',
};

export const CHAIN_TO_RPC: Record<Chain, string> = {
    'facade': 'ws://localhost:12002',
    'solana': 'http://localhost:8899',
};
