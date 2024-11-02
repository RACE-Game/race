// We use this script to serialize raw data into an array

import { BinaryWriter } from '../src/writer';

function write(typ: string, value: string, writer: BinaryWriter) {
  switch (typ) {
    case '-s':
      writer.writeString(value);
      return;
    case '-u8':
      writer.writeU8(Number(value));
      return;
    case '-u16':
      writer.writeU16(Number(value));
      return;
    case '-u32':
      writer.writeU32(Number(value));
      return;
    case '-u64':
      writer.writeU64(BigInt(value));
      return;
    case '-b':
      if (value === 'True' || value === 'true') {
        writer.writeBool(true)
      } else {
        writer.writeBool(false)
      }
      return;
  }
  throw new Error(`Unknown type argument ${typ}`);
}

function main() {
  const args = process.argv.slice(2, process.argv.length);
  let writer = new BinaryWriter();

  if (args.length === 0 || args.length % 2 === 1) {
    console.log(`Serialize raw data and print the result in array format.

Usage:
  npx borsh-serialize [-c|-b|-u8|-u16|-u32|-u64 VALUE]...

Example:
  npx borsh-serialize -s abc -b true -u64 100

Options:
  -s STRING     Append a string
  -u8 INT       Append an integer as u8
  -u16 INT      Append an integer as u16
  -u32 INT      Append an integer as u32
  -u64 INT      Append an integer as u64
  -b BOOL       Append a boolean
`);
    return;
  }

  for (let i = 0; i < args.length; i += 2) {
    const typ = args[i];
    const value = args[i+1];
    write(typ, value, writer);
  }

  console.log('[' + Array.from(writer.toArray()).toString() + ']');
}

main()
