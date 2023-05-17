import { SdkError } from './error';
import { Digest, Secret, Ciphertext } from './types';

let subtle: SubtleCrypto;
let crypto: Crypto;
if (typeof global === 'object') {
  const _crypto = require('crypto');
  crypto = _crypto.webcrypto;
  subtle = _crypto.webcrypto.subtle;
} else {
  crypto = window.crypto;
  subtle = window.crypto.subtle;
}

let aesCounter = crypto.getRandomValues(new Uint8Array(16));

const textDecoder = new TextDecoder('utf8');

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

export async function decryptAes(key: CryptoKey, text: Ciphertext): Promise<Uint8Array> {
  return new Uint8Array(await subtle.decrypt({
    name: "AES-CTR",
    counter: aesCounter,
    length: 128,
  }, key, text));
}

export async function importAes(rawKey: Uint8Array): Promise<CryptoKey> {
  return await subtle.importKey(
    "raw",
    rawKey,
    { name: "AES-CTR" },
    true,
    ["encrypt", "decrypt"]);
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
  signer: string;
  nonce: string;
  timestamp: number;
  signature: string;
}

/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
  addPublicKey(addr: string, raw: string): Promise<void>;

  exportPublicKey(addr?: string): Promise<string>;

  decryptRsa(text: Uint8Array): Promise<Uint8Array>;

  decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext>;

  decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext>;

  sign(message: Uint8Array): Promise<Signature>;

  verify(message: Uint8Array, signature: Signature): Promise<boolean>;

  decryptWithSecrets(ciphertextMap: Map<number, Ciphertext>, secretMap: Map<number, Secret[]>, validOptions: string[]): Promise<Map<number, string>>;
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

  async decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext> {
    const key = await importAes(secret);
    return await decryptAes(key, text);
  }

  async decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext> {
    for (const secret of secrets) {
      text = await this.decryptAes(secret, text);
    }
    return text;
  }

  async sign(message: Uint8Array): Promise<Signature> {
    return {
      signer: "",
      nonce: "",
      timestamp: 1,
      signature: "",
    }
  }

  async verify(message: Uint8Array, signature: Signature): Promise<boolean> {
    return true;
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

  async decryptWithSecrets(ciphertextMap: Map<number, Ciphertext>, secretMap: Map<number, Secret[]>, validOptions: string[]): Promise<Map<number, string>> {
    const res = new Map();
    for (const [idx, ciphertext] of ciphertextMap) {
      const secrets = secretMap.get(idx);
      if (secrets === undefined) {
        throw new Error('Missing secrets');
      } else {
        const decrypted = await this.decryptAesMulti(secrets, ciphertext);
        const decryptedValue = textDecoder.decode(decrypted);
        if (validOptions.find(s => s === decryptedValue) === undefined) {
          throw new Error('Invalid result');
        }
        res.set(idx, decryptedValue);
      }
    }
    return res;
  }
}
