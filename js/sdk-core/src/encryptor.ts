import { SdkError } from './error';
import { Secret, Ciphertext } from './types';
import { field } from '@race-foundation/borsh';
import { base64ToArrayBuffer, arrayBufferToBase64 } from './utils';

let subtle: SubtleCrypto;
if (typeof global === 'object') {
  const _crypto = require('crypto');
  subtle = _crypto.webcrypto.subtle;
} else {
  subtle = window.crypto.subtle;
}

export const aesContentIv = Uint8Array.of(
  0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0);

export const aesDigestIv = Uint8Array.of(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1);

const textDecoder = new TextDecoder('utf8');

const publicExponent = Uint8Array.of(1, 0, 1);

export interface INodePrivateKey {
  rsa: CryptoKeyPair;
  ec: CryptoKeyPair;
}

export interface INodePublicKey {
  rsa: CryptoKey;
  ec: CryptoKey;
}

export interface IPublicKeyRaws {
  rsa: string;
  ec: string;
}

export class PublicKeyRaws {
  @field('string')
  rsa: string;
  @field('string')
  ec: string;
  constructor(fields: IPublicKeyRaws) {
    this.rsa = fields.rsa;
    this.ec = fields.ec;
  }
}

const RSA_PARAMS = {
  name: 'RSA-OAEP',
  hash: 'SHA-256',
};

const EC_PARAMS = {
  name: 'ECDSA',
  namedCurve: 'P-256',
};

export async function exportRsaPublicKey(publicKey: CryptoKey): Promise<string> {
  return arrayBufferToBase64(await subtle.exportKey('spki', publicKey));
}

export async function exportEcPublicKey(publicKey: CryptoKey): Promise<string> {
  return arrayBufferToBase64(await subtle.exportKey('spki', publicKey));
}

export async function exportAes(key: CryptoKey): Promise<Uint8Array> {
  return new Uint8Array(await subtle.exportKey('raw', key));
}

export async function exportRsa(keypair: CryptoKeyPair): Promise<[string, string]> {
  let privkey = await subtle.exportKey('pkcs8', keypair.privateKey);
  return [arrayBufferToBase64(privkey), await exportRsaPublicKey(keypair.publicKey)];
}

export async function exportEc(keypair: CryptoKeyPair): Promise<[string, string]> {
  let privkey = await subtle.exportKey('pkcs8', keypair.privateKey);
  return [arrayBufferToBase64(privkey), await exportEcPublicKey(keypair.publicKey)];
}

export async function encryptRsa(publicKey: CryptoKey, plaintext: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.encrypt('RSA-OAEP', publicKey, plaintext));
}

export async function decryptRsa(privateKey: CryptoKey, ciphertext: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.decrypt('RSA-OAEP', privateKey, ciphertext));
}

export async function signEc(privateKey: CryptoKey, message: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, privateKey, message));
}

export async function verifyEc(publicKey: CryptoKey, signature: Uint8Array, message: Uint8Array): Promise<boolean> {
  return await subtle.verify({ name: 'ECDSA', hash: { name: 'SHA-256' } }, publicKey, signature, message);
}

export async function encryptAes(key: CryptoKey, text: Uint8Array, iv: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(
    await subtle.encrypt(
      {
        name: 'AES-CTR',
        counter: iv,
        length: 64,
      },
      key,
      text
    )
  );
}

export async function decryptAes(key: CryptoKey, text: Ciphertext, iv: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(
    await subtle.decrypt(
      {
        name: 'AES-CTR',
        counter: iv,
        length: 64,
      },
      key,
      text
    )
  );
}

export async function importAes(rawKey: Uint8Array): Promise<CryptoKey> {
  return await subtle.importKey('raw', rawKey, { name: 'AES-CTR' }, true, ['encrypt', 'decrypt']);
}

export async function importRsa([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
  const privateBuf = base64ToArrayBuffer(privateKeyStr);
  const privateKey = await subtle.importKey('pkcs8', privateBuf, RSA_PARAMS, true, ['decrypt']);
  const publicKey = await importRsaPublicKey(publicKeyStr);
  return { publicKey, privateKey };
}

export async function importEc([privateKeyStr, publicKeyStr]: [string, string]): Promise<CryptoKeyPair> {
  const privateBuf = base64ToArrayBuffer(privateKeyStr);
  const privateKey = await subtle.importKey('pkcs8', privateBuf, EC_PARAMS, true, ['sign']);
  const publicKey = await importEcPublicKey(publicKeyStr);
  return { publicKey, privateKey };
}

export async function importRsaPublicKey(publicKeyStr: string): Promise<CryptoKey> {
  const publicBuf = base64ToArrayBuffer(publicKeyStr);
  const publicKey = await subtle.importKey('spki', publicBuf, RSA_PARAMS, true, ['encrypt']);
  return publicKey;
}

export async function importEcPublicKey(publicKeyStr: string): Promise<CryptoKey> {
  const publicBuf = base64ToArrayBuffer(publicKeyStr);
  const publicKey = await subtle.importKey('spki', publicBuf, EC_PARAMS, true, ['verify']);
  return publicKey;
}

export async function generateEcKeypair(): Promise<CryptoKeyPair> {
  return await subtle.generateKey(EC_PARAMS, true, ['verify', 'sign']);
}

export async function generateRsaKeypair(): Promise<CryptoKeyPair> {
  return await subtle.generateKey(
    {
      name: 'RSA-OAEP',
      modulusLength: 1024,
      publicExponent: publicExponent,
      hash: 'SHA-256',
    },
    true,
    ['encrypt', 'decrypt']
  );
}

export async function generateAes(): Promise<CryptoKey> {
  const k = await subtle.generateKey(
    {
      name: 'AES-CTR',
      length: 128,
    },
    true,
    ['encrypt', 'decrypt']
  );
  return k;
}

export interface ISignature {
  signer: string;
  timestamp: bigint;
  signature: Uint8Array;
}

export class Signature {
  @field('string')
  signer: string;
  @field('u64')
  timestamp: bigint;
  @field('u8-array')
  signature: Uint8Array;

  constructor(fields: ISignature) {
    this.signer = fields.signer;
    this.timestamp = fields.timestamp;
    this.signature = fields.signature;
  }
}

/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
  addPublicKey(addr: string, pubkeys: IPublicKeyRaws): Promise<void>;

  exportPublicKey(addr?: string): Promise<IPublicKeyRaws>;

  decryptRsa(text: Uint8Array): Promise<Uint8Array>;

  decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext>;

  decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext>;

  sign(message: Uint8Array, signer: string): Promise<Signature>;

  verify(message: Uint8Array, signature: Signature): Promise<boolean>;

  decryptWithSecrets(
    ciphertextMap: Map<number, Ciphertext>,
    secretMap: Map<number, Secret[]>,
    validOptions: string[]
  ): Promise<Map<number, string>>;
}

class NodePrivateKey implements INodePrivateKey {
  rsa: CryptoKeyPair;
  ec: CryptoKeyPair;

  constructor(rsa: CryptoKeyPair, ec: CryptoKeyPair) {
    this.rsa = rsa;
    this.ec = ec;
  }

  static async initialize(keys?: { rsa?: CryptoKeyPair; ec?: CryptoKeyPair }): Promise<NodePrivateKey> {
    let rsa, ec;
    if (keys?.rsa === undefined) {
      rsa = await generateRsaKeypair();
    } else {
      rsa = keys.rsa;
    }
    if (keys?.ec === undefined) {
      ec = await generateEcKeypair();
    } else {
      ec = keys.ec;
    }
    return new NodePrivateKey(rsa, ec);
  }
}

class NodePublicKey implements INodePublicKey {
  rsa: CryptoKey;
  ec: CryptoKey;

  constructor(rsa: CryptoKey, ec: CryptoKey) {
    this.rsa = rsa;
    this.ec = ec;
  }
}

export class Encryptor implements IEncryptor {
  readonly #privateKey: INodePrivateKey;
  readonly #publicKeys: Map<string, INodePublicKey>;

  constructor(priv: INodePrivateKey) {
    this.#privateKey = priv;
    this.#publicKeys = new Map();
  }

  async decryptRsa(text: Uint8Array): Promise<Uint8Array> {
    return await decryptRsa(this.#privateKey.rsa.privateKey, text);
  }

  async decryptAes(secret: Secret, text: Ciphertext): Promise<Ciphertext> {
    const key = await importAes(secret);
    console.log(aesContentIv);
    return await decryptAes(key, text, aesContentIv);
  }

  async decryptAesMulti(secrets: Secret[], text: Ciphertext): Promise<Ciphertext> {
    for (const secret of secrets) {
      text = await this.decryptAes(secret, text);
    }
    return text;
  }

  async signRaw(message: Uint8Array): Promise<Uint8Array> {
    return await signEc(this.#privateKey.ec.privateKey, message);
  }

  makeSignMessage(message: Uint8Array, timestamp: bigint): Uint8Array {
    const timestampView = new DataView(new ArrayBuffer(8));
    timestampView.setBigUint64(0, timestamp, true);
    const buf = new Uint8Array(message.length + 8);
    buf.set(message);
    buf.set(new Uint8Array(timestampView.buffer), message.length);
    return buf;
  }

  async sign(message: Uint8Array, signer: string): Promise<Signature> {
    const timestamp = BigInt(new Date().getTime());
    const buf = this.makeSignMessage(message, timestamp);
    const signature = await this.signRaw(buf);
    return new Signature({
      timestamp,
      signer,
      signature,
    });
  }

  async verify(message: Uint8Array, signature: Signature): Promise<boolean> {
    const timestamp = signature.timestamp;
    const ecPublicKey = this.#publicKeys.get(signature.signer)?.ec;
    if (ecPublicKey === undefined) {
      throw new Error("Can't verify message, ECDSA key is missing");
    }
    const buf = this.makeSignMessage(message, timestamp);
    return await verifyEc(ecPublicKey, signature.signature, buf);
  }

  static async create(): Promise<Encryptor> {
    const rsaKeypair = await generateRsaKeypair();
    const ecKeypair = await generateEcKeypair();
    return new Encryptor(new NodePrivateKey(rsaKeypair, ecKeypair));
  }

  async exportRsaKeys(): Promise<[string, string]> {
    return await exportRsa(this.#privateKey.rsa);
  }

  async exportEcKeys(): Promise<[string, string]> {
    return await exportEc(this.#privateKey.ec);
  }

  async addPublicKey(addr: string, { rsa, ec }: IPublicKeyRaws): Promise<void> {
    const rsa_ = await importRsaPublicKey(rsa);
    const ec_ = await importEcPublicKey(ec);
    this.#publicKeys.set(addr, new NodePublicKey(rsa_, ec_));
  }

  async exportPublicKey(addr?: string): Promise<IPublicKeyRaws> {
    let rsa, ec;
    if (addr === undefined) {
      rsa = this.#privateKey.rsa.publicKey;
      ec = this.#privateKey.ec.publicKey;
    } else {
      const publicKeys = this.#publicKeys.get(addr);
      if (publicKeys === undefined) {
        throw SdkError.publicKeyNotFound(addr);
      }
      rsa = publicKeys.rsa;
      ec = publicKeys.ec;
    }
    return new PublicKeyRaws({ rsa: await exportRsaPublicKey(rsa), ec: await exportEcPublicKey(ec) });
  }

  async decryptWithSecrets(
    ciphertextMap: Map<number, Ciphertext>,
    secretMap: Map<number, Secret[]>,
    validOptions: string[]
  ): Promise<Map<number, string>> {
    console.log("Ciphertext Map:", ciphertextMap);
    console.log("Secret Map:", secretMap);
    const res = new Map();
    for (const [idx, ciphertext] of ciphertextMap) {
      const secrets = secretMap.get(idx);
      if (secrets === undefined) {
        throw new Error('Missing secrets');
      } else {
        const decrypted = await this.decryptAesMulti(secrets, ciphertext);
        const decryptedValue = textDecoder.decode(decrypted);
        if (validOptions.find(s => s === decryptedValue) === undefined) {
          console.log("Options:", validOptions);
          console.log(decryptedValue);
          throw new Error('Invalid result: [' + decryptedValue + "], options:" + validOptions.join(","));
        }
        res.set(idx, decryptedValue);
      }
    }
    return res;
  }
}
