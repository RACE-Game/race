import { __set_WebSocket_impl } from './connection'
import { __set_subtle_impl } from './crypto'

export function setupNodeEnv() {
    __set_subtle_impl(require('crypto').subtle)
    __set_WebSocket_impl(require('ws'))
}
