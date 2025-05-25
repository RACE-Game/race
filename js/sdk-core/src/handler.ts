import { deserialize, serialize } from '@race-foundation/borsh'
import { GameBundle } from './accounts'
import {
    AnswerDecision,
    GameEvent,
    GameStart,
    Leave,
    Mask,
    Lock,
    SecretsReady,
    ShareSecrets,
    Join,
    Bridge,
} from './events'
import { EventEffects, GameContext } from './game-context'
import { IEncryptor } from './encryptor'
import { Effect } from './effect'
import { Client } from './client'
import { DecryptionCache } from './decryption-cache'
import { InitAccount } from './init-account'

export interface IHandler {
    handleEvent(context: GameContext, event: GameEvent): Promise<EventEffects>

    initState(context: GameContext, initAccount: InitAccount): Promise<EventEffects>
}

export class Handler implements IHandler {
    #encryptor: IEncryptor
    #instance: WebAssembly.Instance
    #client: Client
    #decryptionCache: DecryptionCache

    constructor(
        instance: WebAssembly.Instance,
        encryptor: IEncryptor,
        client: Client,
        decryptionCache: DecryptionCache
    ) {
        this.#encryptor = encryptor
        this.#instance = instance
        this.#client = client
        this.#decryptionCache = decryptionCache
    }

    static async initialize(
        gameBundle: GameBundle,
        encryptor: IEncryptor,
        client: Client,
        decryptionCache: DecryptionCache
    ): Promise<Handler> {
        const importObject = {
            imports: {
                memory: new WebAssembly.Memory({
                    shared: true,
                    maximum: 100,
                    initial: 100,
                }),
            },
        }
        let initiatedSource
        if (gameBundle.data.length === 0) {
            console.debug('Initiate handler by streaming:', gameBundle.uri)
            const data = new Uint8Array(await (await fetch(gameBundle.uri)).arrayBuffer())
            // initiatedSource = await WebAssembly.instantiateStreaming(fetch(gameBundle.uri), importObject)
            initiatedSource = await WebAssembly.instantiate(data, importObject)
        } else {
            initiatedSource = await WebAssembly.instantiate(gameBundle.data, importObject)
        }
        return new Handler(initiatedSource.instance, encryptor, client, decryptionCache)
    }

    async handleEvent(context: GameContext, event: GameEvent): Promise<EventEffects> {
        await this.generalPreHandleEvent(context, event, this.#encryptor)
        const eventEffects = await this.customHandleEvent(context, event)
        await this.generalPostHandleEvent(context, event, eventEffects)
        return eventEffects
    }

    async initState(context: GameContext): Promise<EventEffects> {
        const initAccount = context.initAccount()
        console.info('Initialize game state with:', initAccount)
        context.setTimestamp(0n) // Use 0 timestamp for initState
        await this.generalPreInitState(context, initAccount)
        return await this.customInitState(context, initAccount)
    }

    async generalPreInitState(context: GameContext, _initAccount: InitAccount) {
        context.dispatch = undefined
    }

    async generalPostHandleEvent(_context: GameContext, _event: GameEvent, eventEffects: EventEffects) {
        if (eventEffects.checkpoint !== undefined) {
            this.#decryptionCache.clear()
        }
    }

    async generalPreHandleEvent(context: GameContext, event: GameEvent, encryptor: IEncryptor) {
        if (event instanceof ShareSecrets) {
            const { sender, shares } = event
            const addr = context.idToAddr(sender)
            context.addSharedSecrets(addr, shares)
            let randomIds: number[] = []
            for (let randomState of context.randomStates) {
                if (randomState.status.kind === 'shared') {
                    randomIds.push(randomState.id)
                    randomState.status = { kind: 'ready' }
                }
            }
            if (randomIds.length > 0) {
                context.dispatchEventInstantly(new SecretsReady({ randomIds }))
            }
        } else if (event instanceof AnswerDecision) {
            const { decisionId, ciphertext, sender, digest } = event
            const addr = context.idToAddr(sender)
            context.answerDecision(decisionId, addr, ciphertext, digest)
        } else if (event instanceof Mask) {
            const { sender, randomId, ciphertexts } = event
            const addr = context.idToAddr(sender)
            context.randomizeAndMask(addr, randomId, ciphertexts)
        } else if (event instanceof Lock) {
            const { sender, randomId, ciphertextsAndDigests } = event
            const addr = context.idToAddr(sender)
            context.lock(addr, randomId, ciphertextsAndDigests)
        } else if (event instanceof Join) {
            event.players.forEach(p => context.addPlayer(p))
        } else if (event instanceof Leave) {
        } else if (event instanceof GameStart) {
            context.status = 'running'
            context.setNodeReady(context.accessVersion)
        } else if (event instanceof SecretsReady) {
            for (let randomId of event.randomIds) {
                let decryption = await this.#client.decrypt(context, randomId)
                this.#decryptionCache.add(randomId, decryption)
            }

            for (const st of context.randomStates) {
                const options = st.options
                const revealed = await encryptor.decryptWithSecrets(
                    st.listRevealedCiphertexts(),
                    st.listRevealedSecrets(),
                    options
                )
                context.addRevealedRandom(st.id, revealed)
            }
        } else if (event instanceof Bridge) {
        }
    }

    async customInitState(context: GameContext, initAccount: InitAccount): Promise<EventEffects> {
        const exports = this.#instance.exports
        const mem = exports.memory as WebAssembly.Memory
        mem.grow(4)
        let buf = new Uint8Array(mem.buffer)

        const effect = Effect.fromContext(context, true)

        const effectBytes = serialize(effect)
        const effectSize = effectBytes.length

        const initAccountBytes = serialize(initAccount)
        const initAccountSize = initAccountBytes.length

        // console.debug('Effect Bytes: [%s]', Array.of(effectBytes).toString());

        if (buf.length < 1 + initAccountSize + effectSize) {
            throw new Error(
                `WASM memory overflow, buffer length: ${buf.length}, required: ${1 + initAccountSize + effectSize}`
            )
        }

        let offset = 1
        buf.set(effectBytes, offset)
        offset += effectSize
        buf.set(initAccountBytes, offset)

        const initState = exports.init_state as Function
        const newEffectSize: number = initState(effectSize, initAccountSize)

        switch (newEffectSize) {
            case 0:
                throw new Error('GameBundle: Serializing effect failed')
            case 1:
                console.warn('Effect:', effect)
                throw new Error('GameBundle: Deserializing effect failed')
            case 2:
                console.warn('Event:', initAccount)
                throw new Error('GameBundle: Deserializing init account failed')
        }

        const data = new Uint8Array(mem.buffer)
        const newEffectBytes = data.slice(1, newEffectSize + 1)
        const newEffect = deserialize(Effect, newEffectBytes)

        if (newEffect.error !== undefined) {
            console.error(newEffect.error)
            throw newEffect.error
        } else {
            return await context.applyEffect(newEffect)
        }
    }

    async customHandleEvent(context: GameContext, event: GameEvent): Promise<EventEffects> {
        const exports = this.#instance.exports
        const mem = exports.memory as WebAssembly.Memory
        let buf = new Uint8Array(mem.buffer)

        const effect = Effect.fromContext(context, false)

        const effectBytes = serialize(effect)
        const effectSize = effectBytes.length

        const eventBytes = serialize(event)
        const eventSize = eventBytes.length

        if (buf.length < 1 + eventSize + effectSize) {
            throw new Error(
                `WASM memory overflow, buffer length: ${buf.length}, required: ${1 + eventSize + effectSize}`
            )
        }

        let offset = 1
        buf.set(effectBytes, offset)
        offset += effectSize
        buf.set(eventBytes, offset)

        const handleEvent = exports.handle_event as Function
        const newEffectSize: number = handleEvent(effectSize, eventSize)
        switch (newEffectSize) {
            case 0:
                throw new Error('GameBundle: Serializing effect failed')
            case 1:
                console.warn('Effect:', effect)
                throw new Error('GameBundle: Deserializing effect failed')
            case 2:
                console.warn('Effect:', event)
                throw new Error('GameBundle: Deserializing event failed')
        }
        const data = new Uint8Array(mem.buffer)
        const newEffectBytes = data.slice(1, newEffectSize + 1)

        let newEffect: Effect
        try {
            newEffect = deserialize(Effect, newEffectBytes)
        } catch (err: any) {
            console.error('Failed to deserialize effect, raw: [%s]', Array.from(newEffectBytes).toString())
            throw err
        }

        if (newEffect.error !== undefined) {
            throw newEffect.error
        } else {
            return await context.applyEffect(newEffect)
        }
    }
}
