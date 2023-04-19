export interface IWallet {
  walletAddr: any;
  sendTransaction(tx: any, conn: any): Promise<void>;
}
