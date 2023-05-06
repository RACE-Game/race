/**
 * Encryptor
 * Use RSA and ChaCha20(AES-CTR) for random secrets encryption.
 */

export interface IEncryptor {
  exportPublicKey(addr: string | undefined): string;
}
