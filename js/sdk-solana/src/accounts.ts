import { PublicKey } from '@solana/web3.js';
import * as borsh from 'borsh';
import { ExtendedReader, ExtendedWriter } from './utils'

export class PlayerState {
  isInitialized: boolean;
  nick: string;
  pfp?: PublicKey;

  constructor(data: PlayerState) {
    this.isInitialized = data.isInitialized;
    this.nick = data.nick;
    this.pfp = data.pfp;
  }

  serialize(): Buffer {
    return Buffer.from(borsh.serialize(playerStateSchema, this, ExtendedWriter))
  }

  static deserialize(data: Uint8Array): PlayerState {
    return borsh.deserializeUnchecked(playerStateSchema, PlayerState, Buffer.from(data), ExtendedReader)
  }
}

export const playerStateSchema = new Map([
  [
    PlayerState,
    {
      kind: 'struct',
      fields: [
        ['isInitialized', 'bool'],
        ['nick', 'string'],
        [
          'pfp',
          {
            kind: 'option',
            type: 'publicKey',
          },
        ],
      ],
    },
  ],
]);
