import { Chain } from './common'
import { GameEvent } from './events'

export interface AttachGameParams {
  addr: string
  key: string
}

export interface GetStateParams {
  addr: string
}

export interface SubscribeEventParams {
  addr: string
}

export interface SubmitEventParams {
  addr: string
  event: GameEvent
}

export interface BroadcastFrame {
  gameAddr: string
  state: any
  event: GameEvent
}

export interface IConnection {

  attachGame(params: AttachGameParams): Promise<void>;

  submitEvent(params: SubmitEventParams): Promise<void>;
}
