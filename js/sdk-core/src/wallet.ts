export interface IWallet {
  walletAddr: string;
  sendTransaction(tx: any, conn: any): Promise<void>;
}
