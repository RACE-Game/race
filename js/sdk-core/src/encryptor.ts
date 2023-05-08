import { SdkError } from './error';

let subtle: SubtleCrypto;
let crypto: Crypto;
if (global !== undefined) {
  const _crypto = require('crypto');
  crypto = _crypto.webcrypto;
  subtle = _crypto.webcrypto.subtle;
} else {
  crypto = window.crypto;
  subtle = window.crypto.subtle;
}
console.log("Subtle Crypto:", subtle);

let aesCounter = crypto.getRandomValues(new Uint8Array(16));

const publicExponent = Uint8Array.of(1, 0, 1);

const RSA_PARAMS = {
  name: "RSA-OAEP",
  hash: "SHA-256"
};

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  let binary = '';
  let bytes = new Uint8Array(buffer);
  let len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {

  const rawBytes = atob(base64);
  const uint8Array = new Uint8Array(rawBytes.length);
  for (let i = 0; i < rawBytes.length; i++) {
    uint8Array[i] = rawBytes.charCodeAt(i);
  }
  return uint8Array.buffer;
}

export async function exportRsaPublicKey(publicKey: CryptoKey): Promise<string> {
  return arrayBufferToBase64(await subtle.exportKey("spki", publicKey));
}

export async function exportRsa(keypair: CryptoKeyPair): Promise<[string, string]> {
  let privkey = await subtle.exportKey("pkcs8", keypair.privateKey);
  let pubkey = await subtle.exportKey("spki", keypair.publicKey);
  return [arrayBufferToBase64(privkey), arrayBufferToBase64(pubkey)];
}

export async function encryptRsa(publicKey: CryptoKey, plaintext: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.encrypt("RSA-OAEP", publicKey, plaintext));
}

export async function decryptRsa(publicKey: CryptoKey, ciphertext: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.decrypt("RSA-OAEP", publicKey, ciphertext));
}

export async function encryptAes(key: CryptoKey, text: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.encrypt({
    name: "AES-CTR",
    counter: aesCounter,
    length: 128,
  }, key, text));
}

export async function decryptAes(key: CryptoKey, text: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.decrypt({
    name: "AES-CTR",
    counter: aesCounter,
    length: 128,
  }, key, text));
}

export async function importAes(keyStr: string): Promise<CryptoKey> {
  const key = await subtle.importKey(
    "raw",
    base64ToArrayBuffer(keyStr),
    { name: "AES-CTR" },
    true,
    ["encrypt", "decrypt"]);
  return key;
}

export async function importRsa([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
  const privateBuf = base64ToArrayBuffer(privateKeyStr);
  const privateKey = await subtle.importKey("pkcs8", privateBuf, RSA_PARAMS, true, ["decrypt"]);
  const publicKey = await importRsaPublicKey(publicKeyStr);
  return { publicKey, privateKey }
}

export async function importRsaPublicKey(publicKeyStr: string): Promise<CryptoKey> {
  const publicBuf = base64ToArrayBuffer(publicKeyStr);
  const publicKey = await subtle.importKey("spki", publicBuf, RSA_PARAMS, true, ["encrypt"]);
  return publicKey;
}

export async function generateRsaKeypair(): Promise<CryptoKeyPair> {
  return await subtle.generateKey({
    name: "RSA-OAEP",
    modulusLength: 1024,
    publicExponent: publicExponent,
    hash: "SHA-256"
  }, true, ["encrypt", "decrypt"]);
}

export async function generateAes(): Promise<CryptoKey> {

  const k = await subtle.generateKey({
    name: "AES-CTR",
    length: 128,
  },
    true,
    ["encrypt", "decrypt"]
  );
  return k;
}

export interface Signature {

}


/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
  addPublicKey(addr: string, raw: string): Promise<void>;

  exportPublicKey(addr?: string): Promise<string>;

  decryptRsa(text: Uint8Array): Promise<Uint8Array>;

  decryptAes(secret: Uint8Array, text: Uint8Array): Promise<Uint8Array>;

  sign(message: Uint8Array): Promise<Signature>;

  verify(message: Uint8Array, signature: Signature): Promise<boolean>;
}


export class Encryptor implements IEncryptor {
  readonly #keypair!: CryptoKeyPair;
  readonly #publicKeys: Map<string, CryptoKey>;

  constructor(keypair: CryptoKeyPair) {
    this.#publicKeys = new Map([]);
    this.#keypair = keypair;
  }
  async decryptRsa(text: Uint8Array): Promise<Uint8Array> {
    return await decryptRsa(this.#keypair.privateKey, text);
  }

  async decryptAes(secret: Uint8Array, text: Uint8Array): Promise<Uint8Array> {
    return await decryptAes(secret, text);
  }

  sign(message: Uint8Array): Signature {
    throw new Error('Method not implemented.');
  }

  verify(message: Uint8Array, signature: Signature): boolean {
    throw new Error('Method not implemented.');
  }

  static async default(): Promise<Encryptor> {
    const keypair = await generateRsaKeypair();
    return new Encryptor(keypair);
  }

  async exportKeys(): Promise<[string, string]> {
    return await exportRsa(this.#keypair);
  }

  async addPublicKey(addr: string, raw: string): Promise<void> {
    const publicKey = await importRsaPublicKey(raw);
    this.#publicKeys.set(addr, publicKey);
  }

  exportPublicKey(): Promise<string>;
  exportPublicKey(addr: string): Promise<string>;
  async exportPublicKey(addr?: string): Promise<string> {
    let key;
    if (addr === undefined) {
      key = this.#keypair.publicKey;
    } else {
      key = this.#publicKeys.get(addr);
      if (key === undefined) {
        throw SdkError.publicKeyNotFound(addr);
      }
    }
    return await exportRsaPublicKey(key);
  }
}
