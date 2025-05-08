export class DecryptionCache {
    #caches: Map<number, Map<number, string>>

    constructor() {
        this.#caches = new Map()
    }

    get(randomId: number): Map<number, string> | undefined {
        return this.#caches.get(randomId)
    }

    add(randomId: number, cache: Map<number, string>) {
        this.#caches.set(randomId, cache)
    }

    clear() {
        this.#caches.clear()
    }
}
