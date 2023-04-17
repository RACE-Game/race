import { PublicKey } from "@solana/web3.js";
import * as borsh from 'borsh';

export class ExtendedWriter extends borsh.BinaryWriter {
  writePublicKey(value: PublicKey) {
    let buffer = value.toBuffer();
    this.writeFixedArray(buffer)
  }

  writeBool(value: boolean) {
    this.writeU8(value === true ? 1 : 0)
  }
}

export class ExtendedReader extends borsh.BinaryReader {
  readPublicKey() {
    const value = this.readFixedArray(32);
    return new PublicKey(value)
  }

  readBool() {
    const value = this.readU8();
    return value === 1
  }
}

export class PlayerState {
  isInitialized: boolean;
  nick: string
  pfp?: PublicKey

  constructor(data: PlayerState) {
    this.isInitialized = data.isInitialized;
    this.nick = data.nick;
    this.pfp = data.pfp;
  }
}

export const playerStateSchema =
  new Map([[PlayerState, {
    kind: 'struct',
    fields: [
      ['isInitialized', 'bool'],
      ['nick', 'string'],
      ['pfp', {
        kind: 'option',
        type: 'publicKey',
      }]]
  }]]);
