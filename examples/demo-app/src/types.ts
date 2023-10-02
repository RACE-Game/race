import { Message } from "@race-foundation/sdk-core";

export const CHAIN_VALUES = ['facade', 'solana-local', 'solana-devnet', 'solana-mainnet'] as const;
export type Chain = typeof CHAIN_VALUES[number];

export type IMessage = Message & { id: number };

export function isChain(value: any): value is Chain {
  return CHAIN_VALUES.includes(value)
}
