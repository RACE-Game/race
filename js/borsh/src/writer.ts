import { HasExtendedWriter, IExtendWriter } from './types';

const textEncoder = new TextEncoder();

export interface IWrite {
  length: number;
  write(buf: Uint8Array, offset: number): void;
}

export function writeU8(value: number, buf: Uint8Array, offset: number = 0): number {
  buf.set([value], offset);
  return offset + 1;
}

export function writeU16(value: number, buf: Uint8Array, offset: number = 0): number {
  buf.set([value & 0xff, (value >> 8) & 0xff], offset);
  return offset + 2;
}

export function writeU32(value: number, buf: Uint8Array, offset: number = 0): number {
  buf.set([value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, (value >> 24) & 0xff], offset);
  return offset + 4;
}

export function writeU64(value: bigint, buf: Uint8Array, offset: number = 0): number {
  buf.set(
    [
      Number(value & 0xffn),
      Number((value >> 8n) & 0xffn),
      Number((value >> 16n) & 0xffn),
      Number((value >> 24n) & 0xffn),
      Number((value >> 32n) & 0xffn),
      Number((value >> 40n) & 0xffn),
      Number((value >> 48n) & 0xffn),
      Number((value >> 56n) & 0xffn),
    ],
    offset
  );
  return offset + 8;
}

export class WriteBool implements IWrite {
  #value: boolean;
  constructor(value: boolean) {
    this.#value = value;
  }
  get length() {
    return 1;
  }
  write(buf: Uint8Array, offset: number = 0) {
    buf.set([this.#value ? 1 : 0], offset);
  }
}

export class WriteU8 implements IWrite {
  #value: number;
  constructor(value: number) {
    this.#value = value;
  }
  get length() {
    return 1;
  }
  write(buf: Uint8Array, offset: number = 0) {
    writeU8(this.#value, buf, offset);
  }
}

export class WriteU16 implements IWrite {
  #value: number;
  constructor(value: number) {
    this.#value = value;
  }
  get length() {
    return 2;
  }
  write(buf: Uint8Array, offset: number = 0) {
    writeU16(this.#value, buf, offset);
  }
}

export class WriteU32 implements IWrite {
  #value: number;
  constructor(value: number) {
    this.#value = value;
  }
  get length() {
    return 4;
  }
  write(buf: Uint8Array, offset: number = 0) {
    writeU32(this.#value, buf, offset);
  }
}

export class WriteU64 implements IWrite {
  #value: bigint;
  constructor(value: bigint) {
    this.#value = value;
  }
  get length() {
    return 8;
  }
  write(buf: Uint8Array, offset: number = 0) {
    writeU64(this.#value, buf, offset);
  }
}

export class WriteUint8Array implements IWrite {
  #value: Uint8Array;
  constructor(value: Uint8Array) {
    this.#value = value;
  }
  get length() {
    return this.#value.length;
  }
  write(buf: Uint8Array, offset: number = 0) {
    buf.set(this.#value, offset);
  }
}

export class WriteString {
  #value: Uint8Array;
  #length: number;
  constructor(value: string) {
    this.#value = textEncoder.encode(value);
    this.#length = this.#value.length + 4;
  }
  get length() {
    return this.#length;
  }
  write(buf: Uint8Array, offset: number = 0) {
    // Write length as U32
    offset = writeU32(this.#value.length, buf, offset);
    // Write bytes
    buf.set(this.#value, offset);
  }
}

export class WriteExtended<T> {
  #value: T;
  #length: number;
  #writer: IExtendWriter<T>;
  constructor(value: T, size: number, writer: IExtendWriter<T>) {
    this.#value = value;
    this.#writer = writer;
    this.#length = size;
  }
  get length() {
    return this.#length;
  }
  write(buf: Uint8Array, offset: number = 0) {
    this.#writer.write(this.#value, buf, offset);
  }
}

export class BinaryWriter {
  #writes: IWrite[];
  #length: number;

  constructor() {
    this.#writes = [];
    this.#length = 0;
  }

  writeBool(value: boolean) {
    const w = new WriteBool(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeU8(value: number) {
    const w = new WriteU8(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeU16(value: number) {
    const w = new WriteU16(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeU32(value: number) {
    const w = new WriteU32(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeU64(value: bigint) {
    const w = new WriteU64(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeU128() {
    throw new Error('Not implemented yet!');
  }

  writeI8() {
    throw new Error('Not implemented yet!');
  }

  writeI16() {
    throw new Error('Not implemented yet!');
  }

  writeI32() {
    throw new Error('Not implemented yet!');
  }

  writeI64() {
    throw new Error('Not implemented yet!');
  }

  writeI128() {
    throw new Error('Not implemented yet!');
  }

  writeString(value: string) {
    const w = new WriteString(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeByteArray(value: Uint8Array) {
    const w = new WriteUint8Array(value);
    this.#length += w.length;
    this.#writes.push(w);
  }

  writeF32() {
    throw new Error('Not implemented yet!');
  }

  writeF64() {
    throw new Error('Not implemented yet!');
  }

  writeExtended<T>(value: T, options: HasExtendedWriter<T>) {
    const w = new WriteExtended(value, options.size, options.writer);
    this.#length += w.length;
    this.#writes.push(w);
  }

  toArray(): Uint8Array {
    let buf = new Uint8Array(this.#length);
    let offset = 0;
    for (const w of this.#writes) {
      w.write(buf, offset);
      offset += w.length;
    }
    return buf;
  }
}
