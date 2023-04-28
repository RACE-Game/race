export const CHAIN_VALUES = ['facade', 'solana-local', 'solana-devnet', 'solana-mainnet'] as const;
export type Chain = typeof CHAIN_VALUES[number];

export function isChain(value: any): value is Chain {
  return CHAIN_VALUES.includes(value)
}
