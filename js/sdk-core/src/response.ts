// A class for observable response for client api.
//
// The flow of status in a transaction.
//
// ╔═════════╗  invalid params  ╔══════╗
// ║preparing╟─────────────────▷║failed║
// ╚══════╤══╝                  ╚══════╝
//        │     error in query  ╔══════════════╗
//        ├────────────────────▷║retry-required║
//        ▽                     ╚══════════════╝
// ┏━━━━━━━━━━━━━━━━┓  user cancelled  ╔═════════════╗
// ┃SEND TRANSACTION┠─────────────────▷║user-rejected║
// ┗━┯━━━━━━━━━━━━━━┛                  ╚═════════════╝
//   │ user confirmed ╔══════════╗   error in transaction  ╔══════════════════╗
//   ╰───────────────▷║confirming╟────────────────────────▷║transaction-failed║
//                    ╚══════╤═══╝      ╔═══════╗          ╚══════════════════╝
//                           ╰─────────▷║succeed║
//                                      ╚═══════╝

export type IPendingResponseStatus = {
    status: 'preparing' | 'waiting-wallet'
}

export type ITxPendingResponse = {
    status: 'confirming'
    signature: string
}

export type IOkResponseStatus<T> = {
    status: 'succeed'
    data: T
}

export type IErrResponseStatus<E> = {
    status: 'failed' | 'user-rejected' | 'retry-required'
    error: E
}

export type ITxErrResponseStatus = {
    status: 'transaction-failed'
    error: any // depends on which chain we are at
}

export type ResponseStatus<T, E> = (
    | IPendingResponseStatus
    | ITxPendingResponse
    | IOkResponseStatus<T>
    | IErrResponseStatus<E>
    | ITxErrResponseStatus
)['status']

export type IResponseStatus<T, E> =
    | IPendingResponseStatus
    | ITxPendingResponse
    | IOkResponseStatus<T>
    | IErrResponseStatus<E>
    | ITxErrResponseStatus

export type Response<T, E> =
    | IPendingResponseStatus
    | ITxPendingResponse
    | IErrResponseStatus<E>
    | ITxErrResponseStatus
    | IOkResponseStatus<T>

export type ResponseStream<T, E> = AsyncGenerator<Response<T, E> | undefined>

const isErrStatus = <T, E>(status: ResponseStatus<T, E>) =>
    status === 'failed' || status === 'user-rejected' || status === 'retry-required' || status === 'transaction-failed'

const isEndStatus = <T, E>(status: ResponseStatus<T, E>) => isErrStatus(status) || status === 'succeed'

/**
 * A common type to represent the response of an api call.
 */
export class ResponseHandle<T = void, E = void> {
    current?: Promise<Response<T, E>>
    resolve?: (status: Response<T, E>) => void
    queue: Response<T, E>[]
    status: ResponseStatus<T, E>

    constructor() {
        this.status = 'preparing'
        this.queue = [{ status: 'preparing' }]
    }

    async *stream(): ResponseStream<T, E> {
        while (true) {
            let queueHead = this.queue.shift()
            if (queueHead !== undefined) {
                yield queueHead
                continue
            }
            if (this.resolve === undefined) {
                // Terminate if we have an error status
                if (this.isDone()) {
                    return undefined
                }
                this.current = new Promise(r => (this.resolve = r))
                yield this.current
            } else {
                yield this.current
            }
        }
    }

    isDone() {
        return isEndStatus(this.status)
    }

    isErr() {
        return isErrStatus(this.status)
    }

    update(status: Response<T, E>) {
        if (this.isDone()) {
            return
        }
        if (this.resolve !== undefined) {
            this.resolve(status)
            this.resolve = undefined
        } else {
            this.queue.push(status)
        }
        this.status = status.status
    }

    preparing() {
        this.update({ status: 'preparing' })
    }

    waitingWallet() {
        this.update({ status: 'waiting-wallet' })
    }

    confirming(signature: string) {
        this.update({ status: 'confirming', signature })
    }

    succeed(data: T) {
        this.update({ status: 'succeed', data })
    }

    failed(error: E) {
        this.update({ status: 'failed', error })
    }

    userRejected(error: E) {
        this.update({ status: 'user-rejected', error })
    }

    retryRequired(error: any) {
        this.update({ status: 'retry-required', error })
    }

    transactionFailed(error: any) {
        this.update({ status: 'transaction-failed', error })
    }
}
