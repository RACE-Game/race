import { IConnection } from "./connection";
import { GameContext } from "./game-context";
import { ITransport } from "./transport";
import { IWallet } from "./wallet";


export type EventCallbackFunction =
  (context: GameContext, state: Uint8Array, event: Event | undefined) => void

export class AppClient {
  #addr: string;
  #handler: Handler;
  #wallet: IWallet;
  #transport: ITransport;
  #connection: IConnection;
  #gameContext: GameContext;
  #initGameAccount: InitGameAccount | undefined;
  #callback: EventCallbackFunction;

  constructor(transport: ITransport, wallet: IWallet, gameAddr: string, callback: EventCallbackFunction) {
    this.#transport = transport;
    this.#wallet = wallet;
    this.#addr = gameAddr;
    this.#callback = callback;
  }

  get playerAddr() {
    return this.#wallet.walletAddr
  }

  get gameAddr() {
    return this.#addr;
  }

}
