export const CHAIN_VALUES = ['solana-local', 'solana-devnet', 'solana-mainnet'] as const;
export type Chain = typeof CHAIN_VALUES[number];
