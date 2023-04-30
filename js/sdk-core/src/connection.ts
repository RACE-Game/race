import { Chain } from './common'
import { Event } from './events'

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
  event: Event
}

export interface BroadcastFrame {
  gameAddr: string
  state: any
  event: Event
}

export interface IConnection {

  attachGame(params: AttachGameParams): Promise<void>;

  submitEvent(params: SubmitEventParams): Promise<void>;
}
