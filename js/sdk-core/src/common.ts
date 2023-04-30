export const CHAIN_VALUES = ['facade', 'solana'] as const;

export type Chain = typeof CHAIN_VALUES[number];
