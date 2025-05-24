import { assert } from 'chai';
import { setupNodeEnv } from '../src/node'
import {
  encryptAes,
  decryptAes,
  exportRsa,
  generateAes,
  generateRsaKeypair,
  importRsa,
  decryptRsa,
  encryptRsa,
  generateEcKeypair,
  exportEc,
  importEc,
  signEc,
  verifyEc,
  Encryptor,
  exportAes,
  aesContentIv,
  decryptChacha20,
  encryptChacha20,
  generateChacha20,
  chacha20Nonce,
} from '../src/encryptor';


setupNodeEnv()

describe('Test utilities', () => {
  it('RSA key creation', async () => {
    const keypair = await generateRsaKeypair();
    const keyStrs = await exportRsa(keypair);
    const keypair0 = await importRsa(keyStrs);
    const keyStrs1 = await exportRsa(keypair0);
    assert.deepEqual(keyStrs1, keyStrs);
  });

  it('ECDSA key creation', async () => {
    const keypair = await generateEcKeypair();
    const keyStrs = await exportEc(keypair);
    const keypair0 = await importEc(keyStrs);
    const keyStrs1 = await exportEc(keypair0);
    assert.deepEqual(keyStrs1, keyStrs);
  });

  it('AES-CTR encryption/decryption', async () => {
    const key0 = await generateAes();
    const key1 = await generateAes();
    const data0 = new TextEncoder().encode('OcPShKslbZKO5Gc_H-7WF');
    let data = await encryptAes(key0, data0, aesContentIv);
    data = await encryptAes(key1, data, aesContentIv);
    data = await decryptAes(key1, data, aesContentIv);
    data = await decryptAes(key0, data, aesContentIv);
    assert.deepEqual(data, data0);
  });

  it('Chacha20 encryption/decryption', async () => {
    const key0 = generateChacha20();
    const key1 = generateChacha20();
    const data0 = new TextEncoder().encode('OcPShKslbZKO5Gc_H-7WF');
    let data = encryptChacha20(key0, data0, chacha20Nonce);
    data = encryptChacha20(key1, data, chacha20Nonce);
    data = decryptChacha20(key1, data, chacha20Nonce);
    data = decryptChacha20(key0, data, chacha20Nonce);
    assert.deepEqual(data, data0);
  });

  it('RSA encryption/decryption', async () => {
    const plaintext = Uint8Array.of(1, 2, 3, 4, 5, 6);
    const keypair = await generateRsaKeypair();
    const ciphertext = await encryptRsa(keypair.publicKey, plaintext);
    const decrypted = await decryptRsa(keypair.privateKey, ciphertext);
    assert.deepEqual(decrypted, plaintext);
  });

  it('RSA encrypt AES key', async () => {
    const aes = await generateAes();
    const plain = await exportAes(aes);
    const keypair = await generateRsaKeypair();
    const encrypted = await encryptRsa(keypair.publicKey, plain);
    const decrypted = await decryptRsa(keypair.privateKey, encrypted);
    assert.deepEqual(decrypted, plain);
  });

  it('ECDSA sign/verify', async () => {
    const message = Uint8Array.of(1, 2, 3, 4, 5, 6);
    const keypair = await generateEcKeypair();
    const signature = await signEc(keypair.privateKey, message);
    const result = await verifyEc(keypair.publicKey, signature, message);
    assert.isTrue(result);
  });
});

describe('Test Encryptor', () => {
  it('Test sign/verify', async () => {
    const encryptor = await Encryptor.create('alice', undefined);
    const pubkeys = await encryptor.exportPublicKey();
    encryptor.addPublicKey('alice', pubkeys);
    const message = Uint8Array.of(1, 2, 3, 4, 5, 6);
    const sig = await encryptor.sign(message, 'alice');
    const result = await encryptor.verify(message, sig);
    assert.isTrue(result);
  });

  // it('Test decryptWithSecrets', async () => {
  //   const encryptor = await Encryptor.create();
  //   const ciphertextMap = new Map([
  //     [0, Uint8Array.from([
  //       76, 138, 120, 255, 162, 127, 170, 11, 107, 232, 184, 180, 152, 68, 232, 232, 63,
  //       145, 52, 43, 24,
  //     ])]
  //   ]);
  //   const secretMap = new Map([
  //     [0, [Uint8Array.from([
  //       12, 179, 151, 39, 145, 110, 76, 130, 36, 68, 73, 93, 67, 112, 241, 203,
  //     ])]]
  //   ]);
  //   let decrypted = await encryptor.decryptWithSecrets(ciphertextMap, secretMap, ["OcPShKslbZKO5Gc_H-7WF"]);
  //   assert.deepEqual(decrypted, new Map([[0, "OcPShKslbZKO5Gc_H-7WF"]]));
  // })
});
