
let _subtle: SubtleCrypto | undefined

export function _set_subtle_crypto(s: SubtleCrypto) {
    _subtle = s
}

export function subtle(): SubtleCrypto {
    if (_subtle != undefined) {
        return _subtle
    } else {
        return window.crypto.subtle
    }
}
