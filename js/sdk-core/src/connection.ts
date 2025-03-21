import { IEncryptor, PublicKeyRaws } from './encryptor'
import { GameEvent } from './events'
import { deserialize, enums, field, serialize, struct } from '@race-foundation/borsh'
import { arrayBufferToBase64, base64ToUint8Array } from './utils'
import { BroadcastFrame } from './broadcast-frames'
import { CheckpointOffChain } from './checkpoint'

const PING_TIMEOUT = 6000

export type ConnectionState = 'disconnected' | 'connected' | 'reconnected' | 'closed'

export type AttachResponse = 'success' | 'game-not-loaded'

type Method =
    | 'attach_game'
    | 'submit_event'
    | 'exit_game'
    | 'subscribe_event'
    | 'submit_message'
    | 'get_state'
    | 'ping'
    | 'get_checkpoint'
    | 'get_latest_checkpoint'

interface IAttachGameParams {
    signer: string
    key: PublicKeyRaws
}

interface ISubscribeEventParams {
    settleVersion: bigint
}

interface ISubmitEventParams {
    event: GameEvent
}

interface ISubmitMessageParams {
    content: string
}

interface IGetCheckpointParams {
    settleVersion: bigint
}

export type ConnectionSubscriptionItem = BroadcastFrame | ConnectionState | undefined

export type ConnectionSubscription = AsyncGenerator<ConnectionSubscriptionItem>

export class AttachGameParams {
    @field('string')
    signer: string
    @field(struct(PublicKeyRaws))
    key: PublicKeyRaws

    constructor(fields: IAttachGameParams) {
        this.key = fields.key
        this.signer = fields.signer
    }
}

export class ExitGameParams {
    keepConnection?: boolean
}

export class SubscribeEventParams {
    @field('u64')
    settleVersion: bigint
    constructor(fields: ISubscribeEventParams) {
        this.settleVersion = fields.settleVersion
    }
}

export class SubmitEventParams {
    @field(enums(GameEvent))
    event: GameEvent
    constructor(fields: ISubmitEventParams) {
        this.event = fields.event
    }
}

export class SubmitMessageParams {
    @field('string')
    content: string
    constructor(fields: ISubmitMessageParams) {
        this.content = fields.content
    }
}

export class GetCheckpointParams {
    @field('u64')
    settleVersion: bigint
    constructor(fields: IGetCheckpointParams) {
        this.settleVersion = fields.settleVersion
    }
}

export interface IConnection {
    attachGame(params: AttachGameParams): Promise<AttachResponse>

    startKeepAlive(): void

    getState(): Promise<Uint8Array>

    getCheckpoint(params: GetCheckpointParams): Promise<CheckpointOffChain | undefined>

    submitEvent(params: SubmitEventParams): Promise<ConnectionState | undefined>

    submitMessage(params: SubmitMessageParams): Promise<ConnectionState | undefined>

    exitGame(params: ExitGameParams): Promise<void>

    connect(params: SubscribeEventParams): Promise<void>

    disconnect(): void

    subscribeEvents(): ConnectionSubscription
}

type StreamMessageType = BroadcastFrame | ConnectionState | undefined

export class Connection implements IConnection {
    // The target to connect, in normal game the target is the address
    // of game.  In a sub game, the target is constructed as ADDR:ID.
    target: string
    playerAddr: string
    endpoint: string
    encryptor: IEncryptor
    socket?: WebSocket
    // If the connection is closed
    closed: boolean

    // For async message stream
    streamResolve?: (value: StreamMessageType) => void
    streamMessageQueue: StreamMessageType[]
    streamMessagePromise?: Promise<StreamMessageType>

    // For keep alive
    lastPong: number
    checkTimer?: any

    isFirstOpen: boolean

    constructor(target: string, playerAddr: string, endpoint: string, encryptor: IEncryptor) {
        this.target = target
        this.playerAddr = playerAddr
        this.endpoint = endpoint
        this.encryptor = encryptor
        this.socket = undefined
        this.closed = false
        this.streamResolve = undefined
        this.streamMessageQueue = []
        this.streamMessagePromise = undefined
        this.lastPong = new Date().getTime()
        this.isFirstOpen = true
    }

    onDisconnected() {
        console.warn('Clean up the connection with transactor')

        this.clearCheckTimer()

        if (this.socket === undefined) {
            return
        } else {
            this.socket.close()
            this.socket = undefined
        }

        if (this.streamMessageQueue.find(x => x === 'disconnected') === undefined) {
            if (this.streamResolve !== undefined) {
                let r = this.streamResolve
                this.streamResolve = undefined
                r('disconnected')
            } else {
                this.streamMessageQueue.push('disconnected')
            }
        }
    }

    clearCheckTimer() {
        if (this.checkTimer !== undefined) {
            clearInterval(this.checkTimer)
            this.checkTimer = undefined
        }
    }

    async connect(params: SubscribeEventParams) {
        console.info(`Establishing server connection, target: ${this.target}, settle version: ${params.settleVersion}`)
        this.socket = new WebSocket(this.endpoint)

        this.clearCheckTimer()

        this.socket.onmessage = msg => {
            const frame = this.parseEventMessage(msg.data)
            if (frame !== undefined) {
                if (this.streamResolve !== undefined) {
                    let r = this.streamResolve
                    this.streamResolve = undefined
                    r(frame)
                } else {
                    this.streamMessageQueue.push(frame)
                }
            }
        }

        this.socket.onopen = () => {
            console.info('Websocket connected')
            let frame: ConnectionState
            if (this.isFirstOpen) {
                frame = 'connected'
                this.isFirstOpen = false
            } else {
                frame = 'reconnected'
            }

            if (this.streamResolve !== undefined) {
                let r = this.streamResolve
                this.streamResolve = undefined
                r(frame)
            } else {
                this.streamMessageQueue.push(frame)
            }
        }

        this.socket.onclose = () => {
            console.info('Websocket closed')
            this.closed = true
            this.onDisconnected()
        }

        this.socket.onerror = e => {
            console.error(e)
            this.onDisconnected()
        }

        // Call JSONRPC subscribe_event
        const req = makeReqNoSig(this.target, 'subscribe_event', params)
        await this.requestWs(req)
    }

    startKeepAlive() {
        console.info("Start keep alive task")
        this.clearCheckTimer()
        this.lastPong = new Date().getTime()
        this.checkTimer = setInterval(() => {
            const t = new Date().getTime()
            if (this.lastPong + PING_TIMEOUT < t) {
                console.info('Websocket keep alive check failed, no reply for %s ms', t - this.lastPong)
                this.onDisconnected()
                return
            }
            if (this.socket !== undefined && this.socket.readyState === this.socket.OPEN) {
                this.socket.send(makeReqNoSig(this.target, 'ping', {}))
            }
        }, 3000)
    }

    async attachGame(params: AttachGameParams): Promise<AttachResponse> {
        const req = makeReqNoSig(this.target, 'attach_game', params)
        const resp: any = await this.requestXhr(req)
        if (resp.error !== undefined) {
            return 'game-not-loaded'
        } else {
            return 'success'
        }
    }

    async getState(): Promise<Uint8Array> {
        const req = makeReqNoSig(this.target, 'get_state', {})
        const resp: { result: string } = await this.requestXhr(req)
        return Uint8Array.from(JSON.parse(resp.result))
    }

    async getCheckpoint(params: GetCheckpointParams): Promise<CheckpointOffChain | undefined> {
        const req = makeReqNoSig(this.target, 'get_checkpoint', params)
        const resp: { result: number[] | null } = await this.requestXhr(req)
        if (!resp.result) return undefined
        return CheckpointOffChain.deserialize(Uint8Array.from(resp.result))
    }

    async submitEvent(params: SubmitEventParams): Promise<ConnectionState | undefined> {
        try {
            const req = await this.makeReq(this.target, 'submit_event', params)
            await this.requestXhr(req)
            return undefined
        } catch (_: any) {
            return 'disconnected'
        }
    }

    async submitMessage(params: SubmitMessageParams): Promise<ConnectionState | undefined> {
        try {
            const req = await this.makeReq(this.target, 'submit_message', params)
            await this.requestXhr(req)
            return undefined
        } catch (_: any) {
            return 'disconnected'
        }
    }

    disconnect() {
        if (this.socket !== undefined) {
            this.closed = true
            this.socket.close()
            this.socket = undefined
        }
    }

    async exitGame(params: ExitGameParams): Promise<void> {
        const req = await this.makeReq(this.target, 'exit_game', {})
        await this.requestXhr(req)
        if (!params.keepConnection) this.disconnect()
    }

    async *subscribeEvents(): AsyncGenerator<BroadcastFrame | ConnectionState | undefined> {
        await this.waitSocketReady()
        this.streamMessagePromise = new Promise(r => (this.streamResolve = r))
        while (true) {
            while (this.streamMessageQueue.length > 0) {
                yield this.streamMessageQueue.shift()
            }
            if (this.streamResolve === undefined) {
                this.streamMessagePromise = new Promise(r => (this.streamResolve = r))
                yield this.streamMessagePromise
            } else {
                yield this.streamMessagePromise
            }
        }
    }

    parseEventMessage(raw: string): BroadcastFrame | ConnectionState | undefined {
        try {
            let resp = JSON.parse(raw)
            if (resp.result === 'pong') {
                this.lastPong = new Date().getTime()
                return undefined
            } else if (resp.method === 's_event') {
                if (resp.params.error === undefined) {
                    let result: string = resp.params.result
                    let data = base64ToUint8Array(result)
                    let frame = deserialize(BroadcastFrame, data)
                    return frame
                } else {
                    return 'disconnected'
                }
            } else {
                return undefined
            }
        } catch (e) {
            console.error(`Parse event message error: ${raw}`)
            throw e
        }
    }

    static initialize(target: string, playerAddr: string, endpoint: string, encryptor: IEncryptor): Connection {
        return new Connection(target, playerAddr, endpoint, encryptor)
    }

    async makeReq<P>(target: string, method: Method, params: P): Promise<string> {
        console.debug(`Connection request, target: ${target}, method: ${method}, params:`, params)
        const paramsBytes = serialize(params)
        const sig = await this.encryptor.sign(paramsBytes, this.playerAddr)
        const sigBytes = serialize(sig)
        return JSON.stringify({
            jsonrpc: '2.0',
            method,
            id: crypto.randomUUID(),
            params: [target, arrayBufferToBase64(paramsBytes), arrayBufferToBase64(sigBytes)],
        })
    }

    async requestWs(req: string): Promise<void> {
        try {
            await this.waitSocketReady()
            if (this.socket !== undefined) {
                this.socket.send(req)
            }
        } catch (err) {
            console.error('Failed to connect to current transactor: ' + this.endpoint)
            throw err
        }
    }

    async requestXhr<P>(req: string): Promise<P> {
        try {
            const resp = await fetch(this.endpoint.replace(/^ws/, 'http'), {
                method: 'POST',
                body: req,
                headers: {
                    'Content-Type': 'application/json',
                },
            })
            if (resp.ok) {
                const ret = await resp.json()
                return ret
            } else {
                throw Error('Transactor request failed:' + resp.json())
            }
        } catch (err) {
            console.error('Failed to connect to current transactor: ' + this.endpoint)
            throw err
        }
    }

    waitSocketReady() {
        return new Promise((resolve, reject) => {
            let maxAttempts = 10
            let intervalTime = 200
            let currAttempt = 0
            const interval = setInterval(() => {
                if (currAttempt > maxAttempts) {
                    clearInterval(interval)
                    reject()
                } else if (this.socket !== undefined && this.socket.readyState === this.socket.OPEN) {
                    clearInterval(interval)
                    resolve(undefined)
                }
                currAttempt++
            }, intervalTime)
        })
    }
}

function makeReqNoSig<P>(target: string, method: Method, params: P): string {
    if (method !== 'ping') {
        console.debug(`Connection request[NoSig], target: ${target}, method: ${method}, params:`, params)
    }
    const paramsBytes = serialize(params)
    return JSON.stringify({
        jsonrpc: '2.0',
        method,
        id: crypto.randomUUID(),
        params: [target, arrayBufferToBase64(paramsBytes)],
    })
}

export async function getLatestCheckpoint(
    transactorEndpoint: string,
    addr: string,
): Promise<CheckpointOffChain | undefined> {
    const req = makeReqNoSig(addr, 'get_latest_checkpoint', {})
    try {
        const resp = await fetch(transactorEndpoint.replace(/^ws/, 'http'), {
            method: 'POST',
            body: req,
            headers: {
                'Content-Type': 'application/json',
            },
        })
        if (resp.ok) {
            const ret = await resp.json()
            if (!ret.result) return undefined
            return CheckpointOffChain.deserialize(Uint8Array.from(ret.result))
        } else {
            throw Error('Transactor request failed:' + resp.json())
        }
    } catch (err) {
        console.error('Failed to connect to current transactor')
        throw err
    }
}
