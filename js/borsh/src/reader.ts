import { HasExtendedReader } from './types';

const textDecoder = new TextDecoder('utf8');

export function readU64(buf: Uint8Array, offset: number): bigint {
  const n =
    BigInt(buf[offset]) |
    (BigInt(buf[offset + 1]) << 8n) |
    (BigInt(buf[offset + 2]) << 16n) |
    (BigInt(buf[offset + 3]) << 24n) |
    (BigInt(buf[offset + 4]) << 32n) |
    (BigInt(buf[offset + 5]) << 40n) |
    (BigInt(buf[offset + 6]) << 48n) |
    (BigInt(buf[offset + 7]) << 56n);
  offset += 8;
  return n;
}

export class BinaryReader {
  #buf: Uint8Array;
  #offset: number;

  constructor(buffer: Uint8Array) {
    this.#buf = buffer;
    this.#offset = 0;
  }

  readBool(): boolean {
    const b = this.#buf[this.#offset] == 1;
    this.#offset += 1;
    return b;
  }

  readU8(): number {
    const n = this.#buf[this.#offset];
    this.#offset += 1;
    return n;
  }

  readU16(): number {
    const n = this.#buf[this.#offset] | (this.#buf[this.#offset + 1] << 8);
    this.#offset += 2;
    return n;
  }

  readU32(): number {
    const n =
      this.#buf[this.#offset] |
      (this.#buf[this.#offset + 1] << 8) |
      (this.#buf[this.#offset + 2] << 16) |
      (this.#buf[this.#offset + 3] << 24);
    this.#offset += 4;
    return n;
  }

  readU64(): bigint {
    const n = readU64(this.#buf, this.#offset);
    this.#offset += 8;
    return n;
  }

  readString(): string {
    const length = this.readU32();
    const slice = this.#buf.slice(this.#offset, this.#offset + length);
    this.#offset += length;
    return textDecoder.decode(slice);
  }

  readByteArray(length: number): Uint8Array {
    const slice = this.#buf.slice(this.#offset, this.#offset + length);
    this.#offset += length;
    return slice;
  }

  readExtended<T>(options: HasExtendedReader<T>): T {
    const value = options.reader.read(this.#buf, this.#offset);
    this.#offset += options.size;
    return value;
  }
}
