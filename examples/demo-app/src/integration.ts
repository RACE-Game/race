import React, { FC } from 'react';
import { useContext } from 'react';
import * as SolWalletAdapter from '@solana/wallet-adapter-react';
import { Chain } from './types';
import FacadeWalletWrapper from './FacadeWalletWrapper';
import SolanaWalletWrapper from './SolanaWalletWrapper';
import { ITransport, IWallet } from 'race-sdk-core';
import { SolanaTransport, SolanaWalletAdapter } from 'race-sdk-solana';
import { FacadeWalletContext } from './facade-wallet-context';
import { FacadeTransport } from 'race-sdk-facade';

export function useWallet(chain: Chain): IWallet {
  switch (chain) {
    case 'facade':
      return useContext(FacadeWalletContext);
    case 'solana-local':
    case 'solana-devnet':
    case 'solana-mainnet':
      return new SolanaWalletAdapter(SolWalletAdapter.useWallet());
  }
}

export function getWalletWrapper(chain: Chain): FC<{ children: React.ReactNode }> {
  switch (chain) {
    case 'facade':
      return FacadeWalletWrapper;
    case 'solana-local':
    case 'solana-devnet':
    case 'solana-mainnet':
      return SolanaWalletWrapper;
  }
}

export function createTransport(chain: Chain, url: string): ITransport {
  console.log("Create transport: %s", chain);
  switch (chain) {
    case 'facade':
      return new FacadeTransport(url);
    case 'solana-local':
    case 'solana-devnet':
    case 'solana-mainnet':
      return new SolanaTransport(url);
  }
}
