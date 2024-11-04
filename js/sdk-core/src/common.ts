export const CHAIN_VALUES = ['facade', 'solana'] as const

export type Chain = (typeof CHAIN_VALUES)[number]

export const TOKEN_CACHE_TTL = 3600 * 24 * 30 // Month
export const NFT_CACHE_TTL = 3600 * 24 * 30 // Month
