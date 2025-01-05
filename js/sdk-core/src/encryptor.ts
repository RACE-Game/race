import { SdkError } from './error'
import { Secret, Ciphertext } from './types'
import { field } from '@race-foundation/borsh'
import { base64ToArrayBuffer, arrayBufferToBase64 } from './utils'
import { Chacha20 } from 'ts-chacha20'
import { IStorage } from './storage'
import { subtle } from './crypto'

const ENCRYPTOR_VERSION = '1.0'

export const aesContentIv = Uint8Array.of(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)

export const chacha20Nonce = Uint8Array.of(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)

export const aesDigestIv = Uint8Array.of(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1)

const textDecoder = new TextDecoder('utf8')

const publicExponent = Uint8Array.of(1, 0, 1)

export interface INodePrivateKey {
    rsa: CryptoKeyPair
    ec: CryptoKeyPair
}

export interface INodePublicKey {
    rsa: CryptoKey
    ec: CryptoKey
}

export interface IPublicKeyRaws {
    rsa: string
    ec: string
}

export class PublicKeyRaws {
    @field('string')
    rsa: string
    @field('string')
    ec: string
    constructor(fields: IPublicKeyRaws) {
        this.rsa = fields.rsa
        this.ec = fields.ec
    }
}

const RSA_PARAMS = {
    name: 'RSA-OAEP',
    hash: 'SHA-256',
}

const EC_PARAMS = {
    name: 'ECDSA',
    namedCurve: 'P-256',
}

export async function exportRsaPublicKey(publicKey: CryptoKey): Promise<string> {
    return arrayBufferToBase64(await subtle().exportKey('spki', publicKey))
}

export async function exportEcPublicKey(publicKey: CryptoKey): Promise<string> {
    return arrayBufferToBase64(await subtle().exportKey('spki', publicKey))
}

export async function exportAes(key: CryptoKey): Promise<Uint8Array> {
    return new Uint8Array(await subtle().exportKey('raw', key))
}

export async function exportRsa(keypair: CryptoKeyPair): Promise<[string, string]> {
    let privkey = await subtle().exportKey('pkcs8', keypair.privateKey)
    return [arrayBufferToBase64(privkey), await exportRsaPublicKey(keypair.publicKey)]
}

export async function exportEc(keypair: CryptoKeyPair): Promise<[string, string]> {
    let privkey = await subtle().exportKey('pkcs8', keypair.privateKey)
    return [arrayBufferToBase64(privkey), await exportEcPublicKey(keypair.publicKey)]
}

export async function encryptRsa(publicKey: CryptoKey, plaintext: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(await subtle().encrypt('RSA-OAEP', publicKey, plaintext))
}

export async function decryptRsa(privateKey: CryptoKey, ciphertext: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(await subtle().decrypt('RSA-OAEP', privateKey, ciphertext))
}

export async function signEc(privateKey: CryptoKey, message: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(await subtle().sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, privateKey, message))
}

export async function verifyEc(publicKey: CryptoKey, signature: Uint8Array, message: Uint8Array): Promise<boolean> {
    return await subtle().verify({ name: 'ECDSA', hash: { name: 'SHA-256' } }, publicKey, signature, message)
}

export function encryptChacha20(key: Uint8Array, text: Uint8Array, nonce: Uint8Array): Uint8Array {
    return new Chacha20(key, nonce).encrypt(text)
}

export function decryptChacha20(key: Uint8Array, text: Uint8Array, nonce: Uint8Array): Uint8Array {
    return new Chacha20(key, nonce).decrypt(text)
}

export async function encryptAes(key: CryptoKey, text: Uint8Array, iv: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(
        await subtle().encrypt(
            {
                name: 'AES-CTR',
                counter: iv,
                length: 64,
            },
            key,
            text
        )
    )
}

export async function decryptAes(key: CryptoKey, text: Ciphertext, iv: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(
        await subtle().decrypt(
            {
                name: 'AES-CTR',
                counter: iv,
                length: 64,
            },
            key,
            text
        )
    )
}

export async function importAes(rawKey: Uint8Array): Promise<CryptoKey> {
    return await subtle().importKey('raw', rawKey, { name: 'AES-CTR' }, true, ['encrypt', 'decrypt'])
}

export async function importRsa([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
    const privateBuf = base64ToArrayBuffer(privateKeyStr)
    const privateKey = await subtle().importKey('pkcs8', privateBuf, RSA_PARAMS, true, ['decrypt'])
    const publicKey = await importRsaPublicKey(publicKeyStr)
    return { publicKey, privateKey }
}

export async function importEc([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
    const privateBuf = base64ToArrayBuffer(privateKeyStr)
    const privateKey = await subtle().importKey('pkcs8', privateBuf, EC_PARAMS, true, ['sign'])
    const publicKey = await importEcPublicKey(publicKeyStr)
    return { publicKey, privateKey }
}

export async function importRsaPublicKey(publicKeyStr: string): Promise<CryptoKey> {
    const publicBuf = base64ToArrayBuffer(publicKeyStr)
    const publicKey = await subtle().importKey('spki', publicBuf, RSA_PARAMS, true, ['encrypt'])
    return publicKey
}

export async function importEcPublicKey(publicKeyStr: string): Promise<CryptoKey> {
    const publicBuf = base64ToArrayBuffer(publicKeyStr)
    const publicKey = await subtle().importKey('spki', publicBuf, EC_PARAMS, true, ['verify'])
    return publicKey
}

export async function generateEcKeypair(): Promise<CryptoKeyPair> {
    return await subtle().generateKey(EC_PARAMS, true, ['verify', 'sign'])
}

export async function generateRsaKeypair(): Promise<CryptoKeyPair> {
    return await subtle().generateKey(
        {
            name: 'RSA-OAEP',
            modulusLength: 1024,
            publicExponent: publicExponent,
            hash: 'SHA-256',
        },
        true,
        ['encrypt', 'decrypt']
    )
}

export function generateChacha20(): Uint8Array {
    const arr = new Uint8Array(32)
    crypto.getRandomValues(arr)
    return arr
}

export async function generateAes(): Promise<CryptoKey> {
    const k = await subtle().generateKey(
        {
            name: 'AES-CTR',
            length: 128,
        },
        true,
        ['encrypt', 'decrypt']
    )
    return k
}

export interface ISignature {
    signer: string
    timestamp: bigint
    signature: Uint8Array
}

export class Signature {
    @field('string')
    signer: string
    @field('u64')
    timestamp: bigint
    @field('u8-array')
    signature: Uint8Array

    constructor(fields: ISignature) {
        this.signer = fields.signer
        this.timestamp = fields.timestamp
        this.signature = fields.signature
    }
}

/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
    addPublicKey(addr: string, pubkeys: IPublicKeyRaws): Promise<void>

    exportPublicKey(addr?: string): Promise<IPublicKeyRaws>

    decryptRsa(text: Uint8Array): Promise<Uint8Array>

    decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext>

    decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext>

    encryptChacha20(secret: Secret, text: Ciphertext): Ciphertext

    decryptChacha20(secret: Secret, text: Ciphertext): Ciphertext

    sign(message: Uint8Array, signer: string): Promise<Signature>

    verify(message: Uint8Array, signature: Signature): Promise<boolean>

    decryptWithSecrets(
        ciphertextMap: Map<number, Ciphertext>,
        secretMap: Map<number, Secret[]>,
        validOptions: string[]
    ): Promise<Map<number, string>>
}

class NodePrivateKey implements INodePrivateKey {
    rsa: CryptoKeyPair
    ec: CryptoKeyPair

    constructor(rsa: CryptoKeyPair, ec: CryptoKeyPair) {
        this.rsa = rsa
        this.ec = ec
    }

    static async initialize(keys?: { rsa?: CryptoKeyPair; ec?: CryptoKeyPair }): Promise<NodePrivateKey> {
        let rsa, ec
        if (keys?.rsa === undefined) {
            rsa = await generateRsaKeypair()
        } else {
            rsa = keys.rsa
        }
        if (keys?.ec === undefined) {
            ec = await generateEcKeypair()
        } else {
            ec = keys.ec
        }
        return new NodePrivateKey(rsa, ec)
    }
}

class NodePublicKey implements INodePublicKey {
    rsa: CryptoKey
    ec: CryptoKey

    constructor(rsa: CryptoKey, ec: CryptoKey) {
        this.rsa = rsa
        this.ec = ec
    }
}

export class Encryptor implements IEncryptor {
    readonly #privateKey: INodePrivateKey
    readonly #publicKeys: Map<string, INodePublicKey>

    constructor(priv: INodePrivateKey) {
        this.#privateKey = priv
        this.#publicKeys = new Map()
    }

    async decryptRsa(text: Uint8Array): Promise<Uint8Array> {
        return await decryptRsa(this.#privateKey.rsa.privateKey, text)
    }

    async decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext> {
        const key = await importAes(secret)
        return await decryptAes(key, text, aesContentIv)
    }

    async decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext> {
        for (const secret of secrets) {
            text = await this.decryptAes(secret, text)
        }
        return text
    }

    encryptChacha20(secret: Secret, text: Ciphertext): Ciphertext {
        return encryptChacha20(secret, text, chacha20Nonce)
    }

    decryptChacha20(secret: Secret, text: Ciphertext): Ciphertext {
        return decryptChacha20(secret, text, chacha20Nonce)
    }

    decryptChacha20Multi(secrets: Secret[], text: Ciphertext): Ciphertext {
        for (const secret of secrets) {
            text = this.decryptChacha20(secret, text)
        }
        return text
    }

    async signRaw(message: Uint8Array): Promise<Uint8Array> {
        return await signEc(this.#privateKey.ec.privateKey, message)
    }

    makeSignMessage(message: Uint8Array, timestamp: bigint): Uint8Array {
        const timestampView = new DataView(new ArrayBuffer(8))
        timestampView.setBigUint64(0, timestamp, true)
        const buf = new Uint8Array(message.length + 8)
        buf.set(message)
        buf.set(new Uint8Array(timestampView.buffer), message.length)
        return buf
    }

    async sign(message: Uint8Array, signer: string): Promise<Signature> {
        const timestamp = BigInt(new Date().getTime())
        const buf = this.makeSignMessage(message, timestamp)
        const signature = await this.signRaw(buf)
        return new Signature({
            timestamp,
            signer,
            signature,
        })
    }

    async verify(message: Uint8Array, signature: Signature): Promise<boolean> {
        const timestamp = signature.timestamp
        const ecPublicKey = this.#publicKeys.get(signature.signer)?.ec
        if (ecPublicKey === undefined) {
            throw new Error("Can't verify message, ECDSA key is missing")
        }
        const buf = this.makeSignMessage(message, timestamp)
        return await verifyEc(ecPublicKey, signature.signature, buf)
    }

    static async create(playerAddr: string, storage: IStorage | undefined): Promise<Encryptor> {
        if (storage !== undefined) {
            const imported = await Encryptor.importFromStorage(playerAddr, storage)
            if (imported !== undefined) {
                return imported
            }
        }
        const rsaKeypair = await generateRsaKeypair()
        const ecKeypair = await generateEcKeypair()
        const encryptor = new Encryptor(new NodePrivateKey(rsaKeypair, ecKeypair))
        if (storage !== undefined) {
            await encryptor.exportToStorage(playerAddr, storage)
        }
        return encryptor
    }

    static makeStorageKey(playerAddr: string): string {
        return `ENCRYPTOR_KEY_${ENCRYPTOR_VERSION}_${playerAddr}`
    }

    async exportToStorage(playerAddr: string, storage: IStorage) {
        const ec = await this.exportEcKeys()
        const rsa = await this.exportRsaKeys()
        storage.setItem(
            Encryptor.makeStorageKey(playerAddr),
            JSON.stringify({
                rsa,
                ec,
            })
        )
    }

    static async importFromStorage(playerAddr: string, storage: IStorage): Promise<Encryptor | undefined> {
        const k = Encryptor.makeStorageKey(playerAddr)
        const v = storage.getItem(k)
        if (v === null) {
            return undefined
        }
        const { rsa, ec }: { rsa: [string, string]; ec: [string, string] } = JSON.parse(v)
        const ecKeypair = await importEc(ec)
        const rsaKeypair = await importRsa(rsa)
        return new Encryptor(new NodePrivateKey(rsaKeypair, ecKeypair))
    }

    async exportRsaKeys(): Promise<[string, string]> {
        return await exportRsa(this.#privateKey.rsa)
    }

    async exportEcKeys(): Promise<[string, string]> {
        return await exportEc(this.#privateKey.ec)
    }

    async addPublicKey(addr: string, { rsa, ec }: IPublicKeyRaws): Promise<void> {
        const rsa_ = await importRsaPublicKey(rsa)
        const ec_ = await importEcPublicKey(ec)
        this.#publicKeys.set(addr, new NodePublicKey(rsa_, ec_))
    }

    async exportPublicKey(addr?: string): Promise<IPublicKeyRaws> {
        let rsa, ec
        if (addr === undefined) {
            rsa = this.#privateKey.rsa.publicKey
            ec = this.#privateKey.ec.publicKey
        } else {
            const publicKeys = this.#publicKeys.get(addr)
            if (publicKeys === undefined) {
                throw SdkError.publicKeyNotFound(addr)
            }
            rsa = publicKeys.rsa
            ec = publicKeys.ec
        }
        return new PublicKeyRaws({
            rsa: await exportRsaPublicKey(rsa),
            ec: await exportEcPublicKey(ec),
        })
    }

    async decryptWithSecrets(
        ciphertextMap: Map<number, Ciphertext>,
        secretMap: Map<number, Secret[]>,
        validOptions: string[]
    ): Promise<Map<number, string>> {
        const res = new Map()
        for (const [idx, ciphertext] of ciphertextMap) {
            const secrets = secretMap.get(idx)
            if (secrets === undefined) {
                throw new Error('Missing secrets')
            } else {
                const decrypted = this.decryptChacha20Multi(secrets, ciphertext)
                const decryptedValue = textDecoder.decode(decrypted)
                if (validOptions.find(s => s === decryptedValue) === undefined) {
                    throw new Error('Invalid result: [' + decryptedValue + '], options:' + validOptions.join(','))
                }
                res.set(idx, decryptedValue)
            }
        }
        return res
    }
}

export async function sha256(data: Uint8Array): Promise<Uint8Array> {
    let hashBuffer = await subtle().digest('SHA-256', data)
    return new Uint8Array(hashBuffer)
}

export async function sha256String(data: Uint8Array): Promise<string> {
    return Array.from(await sha256(data))
        .map(byte => byte.toString(16).padStart(2, '0'))
        .join('')
}
