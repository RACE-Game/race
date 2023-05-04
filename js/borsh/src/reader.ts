
const textDecoder = new TextDecoder('utf8');

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
    const n = this.#buf[this.#offset] + this.#buf[this.#offset + 1] << 8;
    this.#offset += 2;
    return n;
  }

  readU32(): number {
    const n = this.#buf[this.#offset]
      + this.#buf[this.#offset + 1] << 8
      + this.#buf[this.#offset + 2] << 16
      + this.#buf[this.#offset + 3] << 24;
    this.#offset += 4;
    return n;
  }

  readU64(): bigint {
    const n = BigInt(this.#buf[this.#offset])
      + BigInt(this.#buf[this.#offset + 1] << 8)
      + BigInt(this.#buf[this.#offset + 2] << 16)
      + BigInt(this.#buf[this.#offset + 3] << 24)
      + BigInt(this.#buf[this.#offset + 4] << 32)
      + BigInt(this.#buf[this.#offset + 5] << 40)
      + BigInt(this.#buf[this.#offset + 6] << 48)
      + BigInt(this.#buf[this.#offset + 7] << 56);
    this.#offset += 8;
    return n;
  }

  readString(): string {
    const length = this.readU32();
    const slice = this.#buf.slice(this.#offset, this.#offset + length);
    this.#offset += length;
    return textDecoder.decode(slice)
  }

  readByteArray(length: number): Uint8Array {
    const slice = this.#buf.slice(this.#offset, this.#offset + length);
    this.#offset += length;
    return slice;
  }
}
