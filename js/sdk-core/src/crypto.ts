let __subtle_impl: SubtleCrypto | undefined = undefined

export function __set_subtle_impl(subtle: SubtleCrypto) {
    __subtle_impl = subtle
}

export function subtle(): SubtleCrypto {
    if (__subtle_impl === undefined && (typeof window === 'undefined')) {
        throw new Error('No subtle crypto available. Call `setupNodeEnv()` to configure it.')
    } else if (__subtle_impl) {
        return __subtle_impl
    } else {
        return window.crypto.subtle
    }
}
