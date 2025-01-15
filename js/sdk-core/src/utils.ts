export function arrayBufferToBase64(buffer: ArrayBuffer): string {
    let binary = ''
    let bytes = new Uint8Array(buffer)
    let len = bytes.byteLength
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i])
    }
    return btoa(binary)
}

export function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const rawBytes = atob(base64)
    const uint8Array = new Uint8Array(rawBytes.length)
    for (let i = 0; i < rawBytes.length; i++) {
        uint8Array[i] = rawBytes.charCodeAt(i)
    }
    return uint8Array.buffer
}

export function base64ToUint8Array(base64: string): Uint8Array {
    const rawBytes = atob(base64)
    const uint8Array = new Uint8Array(rawBytes.length)
    for (let i = 0; i < rawBytes.length; i++) {
        uint8Array[i] = rawBytes.charCodeAt(i)
    }
    return uint8Array
}
