


export interface IWrite {}

export class WriteU8 {
  #data: Uint8Array;

}

export class BinaryWriter {
  #buf: Uint8Array;
  #writes: IWrite[];

  writeBool() {

  }

  writeU8(value: number) {

  }

  writeU16() {

  }

  writeU32() {

  }

  writeU64() {

  }

  writeU128() {

  }

  writeI8() {

  }

  writeI16() {

  }

  writeI32() {

  }

  writeI64() {

  }

  writeI128() {

  }

  writeString() {

  }

  writeUint8Array() {

  }

  writeF32() {

  }

  writeF64() {

  }

  writeArray() {

  }

  static writeU64(value: bigint, writer: BinaryWriter) {

  }

  toArray(): Uint8Array {
    return Uint8Array.of();
  }
}

export class BinaryReader {

}
