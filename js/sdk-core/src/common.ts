export const CHAIN_VALUES = ['facade', 'solana', 'sui'] as const

export type Chain = (typeof CHAIN_VALUES)[number]

export const TOKEN_CACHE_TTL = 3600 * 24 * 30 // 1 month
export const NFT_CACHE_TTL = 3600 * 24 * 30 // 1 month
export const GAME_ACCOUNT_CACHE_TTL = 3600 // 1 hour
export const BUNDLE_CACHE_TTL = 3600 * 24 * 30 // 1 month
