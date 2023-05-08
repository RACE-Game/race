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

export async function rsaEncrypt(publicKey: CryptoKey, plaintext: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.encrypt("RSA-OAEP", publicKey, plaintext));
}

export async function rsaDecrypt(publicKey: CryptoKey, ciphertext: Uint8Array): Promise<Uint8Array> {
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

export async function importRsa([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
  const algorithm = {
    name: "RSA-OAEP",
    hash: "SHA-256"
  };
  const privateBuf = base64ToArrayBuffer(privateKeyStr);
  const publicBuf = base64ToArrayBuffer(publicKeyStr);
  const privateKey = await subtle.importKey("pkcs8", privateBuf, algorithm, true, ["decrypt"]);
  const publicKey = await subtle.importKey("spki", publicBuf, algorithm, true, ["encrypt"]);
  return { publicKey, privateKey }
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

/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
  exportPublicKey(addr?: string): string;
}


export class Encryptor {
  readonly #keypair!: CryptoKeyPair;
  readonly #publicKeys: Map<string, CryptoKey>;

  constructor(keypair: CryptoKeyPair) {
    this.#publicKeys = new Map([]);
    this.#keypair = keypair;
  }

  static async default(): Promise<Encryptor> {
    const keypair = await generateRsaKeypair();
    return new Encryptor(keypair);
  }

  // static async fromPrivateKey(key: string): Promise<Encryptor> {

  // }
  // static async fromPem(pem: string): Promise<Encryptor> {
  //   const privateKey = await importRsaPrivateKeyFromPem(pem);
  //   const publicKey = await getRsaPublicKey(privateKey);
  //   const keypair = { publicKey, privateKey };
  //   return new Encryptor(keypair);
  // }

  async exportKeys(): Promise<[string, string]> {
    return await exportRsa(this.#keypair);
  }

  exportPublicKey(): Promise<string>;
  exportPublicKey(addr: string): Promise<string>;
  async exportPublicKey(addr?: string): Promise<string> {
    // let key;
    // if (addr === undefined) {
    //   key = this.#defaultPublicKey;
    // } else {
    //   key = this.#publicKeys.get(addr);
    //   if (key === undefined) {
    //     throw SdkError.publicKeyNotFound(addr);
    //   }
    // }
    // let exported = await subtle.exportKey("spki", key);
    // let body = window.btoa(String.fromCharCode(...new Uint8Array(exported)));
    // body = body.match(/.{1,64}/g)!.join('\n');
    // return `${PEM_HEADER}\n${body}\n${PEM_FOOTER}`;
    return '';
  }
}
