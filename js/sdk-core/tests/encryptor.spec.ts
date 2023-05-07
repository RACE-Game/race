import { assert } from 'chai';
import { Encryptor, exportRsa, generateRsaKeypair, importRsa, rsaDecrypt, rsaEncrypt } from '../src/encryptor';

describe('Test utilities', () => {
  it('RSA key creation', async () => {
    const keypair = await generateRsaKeypair();
    const keyStrs = await exportRsa(keypair);
    const keypair0 = await importRsa(keyStrs);
    const keyStrs1 = await exportRsa(keypair0);
    console.log(keyStrs[0]);
    console.log(keyStrs[1]);
    assert.deepEqual(keyStrs1, keyStrs);
  });

  it('AES-CTR key creation', async () => {

  });

  it('RSA encryption/decryption', async () => {
    const plaintext = Uint8Array.of(1, 2, 3, 4, 5, 6);
    const keypair = await generateRsaKeypair();
    const ciphertext = await rsaEncrypt(keypair.publicKey, plaintext);
    const decrypted = await rsaDecrypt(keypair.privateKey, ciphertext);
    assert.deepEqual(decrypted, plaintext);
  })
});
