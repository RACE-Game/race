import { Address, Chain } from "./common";
import { Event } from "./event";

export type AttachGameParams = {
    addr: Address,
    chain: Chain,
};

export type GetStateParams = {
    addr: Address,
};

export type SubscribeEventParams = {
    addr: Address,
}

export type SubmitEventParams = {
    addr: Address,
    event: Event
};

export type BroadcastFrame = {
    gameAddr: Address,
    state: any,
    event: Event,
}
