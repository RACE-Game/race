import { TransactionResult } from "./transport";

export interface IWallet {
  isConnected: boolean;
  walletAddr: string;
  sendTransaction(tx: any, conn: any, config?: any): Promise<TransactionResult<void>>;
}
