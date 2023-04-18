import { PublicKey } from '@solana/web3.js';
import * as borsh from 'borsh';

export class ExtendedWriter extends borsh.BinaryWriter {
  writePublicKey(value: PublicKey) {
    let buffer = value.toBuffer();
    this.writeFixedArray(buffer);
  }

  writeBool(value: boolean) {
    this.writeU8(value === true ? 1 : 0);
  }

  writeBytes(value: Uint8Array) {
    this.writeU32(value.length)
    this.writeFixedArray(value)
  }

  writeU64(value: bigint) {
    super.writeU64(value)
  }
}

export class ExtendedReader extends borsh.BinaryReader {
  readPublicKey() {
    const value = this.readFixedArray(32);
    return new PublicKey(value);
  }

  readBool() {
    const value = this.readU8();
    return value === 1;
  }

  readU64() {
    return Buffer.from(this.readFixedArray(8)).readBigUInt64LE(0)
  }

  readBytes() {
    const len = this.readU32();
    return this.readFixedArray(len);
  }
}
