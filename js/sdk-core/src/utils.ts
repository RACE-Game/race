import * as borsh from 'borsh';

export class ExtendedWriter extends borsh.BinaryWriter {
  writeBool(value: boolean) {
    this.writeU8(value === true ? 1 : 0);
  }

  writeBytes(value: Uint8Array) {
    this.writeU32(value.length);
    this.writeFixedArray(value);
  }

  writeBigint(value: bigint) {
    let buf = Buffer.alloc(8);
    buf.writeBigUInt64LE(value);
    super.writeFixedArray(Uint8Array.from(buf));
  }
}

export class ExtendedReader extends borsh.BinaryReader {
  readBool() {
    const value = this.readU8();
    return value === 1;
  }

  readBigint() {
    let arr = this.readFixedArray(8);
    let buf = Buffer.from(arr);
    return buf.readBigUInt64LE();
  }

  readBytes() {
    const len = this.readU32();
    return this.readFixedArray(len);
  }
}