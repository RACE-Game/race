import { Address, Chain } from './common'
import { Event } from './event'

export interface AttachGameParams {
  addr: Address
  chain: Chain
}

export interface GetStateParams {
  addr: Address
}

export interface SubscribeEventParams {
  addr: Address
}

export interface SubmitEventParams {
  addr: Address
  event: Event
}

export interface BroadcastFrame {
  gameAddr: Address
  state: any
  event: Event
}
