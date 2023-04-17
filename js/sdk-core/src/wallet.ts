export interface IWallet {
  walletAddr: string;
  sendTransaction(tx: any): Promise<void>;
}
