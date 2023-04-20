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

  writeBigint(value: bigint) {
    let buf = Buffer.alloc(8);
    buf.writeBigUInt64LE(value);
    super.writeFixedArray(Uint8Array.from(buf))
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

  readBigint() {
    let arr = this.readFixedArray(8)
    let buf = Buffer.from(arr);
    return buf.readBigUInt64LE();
  }

  readBytes() {
    const len = this.readU32();
    return Array.from(this.readFixedArray(len));
  }
}
