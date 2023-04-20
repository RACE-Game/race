import { PublicKey } from '@solana/web3.js';
import { ExtendedReader } from './utils';
import * as borsh from 'borsh';

/**
 * A partial port of Metaplex's Metadata layout.
 *
 * Metaplex library introduces extra dependencies that requires node
 * polyfill, And we only use a small set of its features.
 */
export interface IMetadata {
  key: number;
  updateAuthority: PublicKey;
  mint: PublicKey;
  data: Data;
}

export interface ICreator {
  address: PublicKey;
  verified: boolean;
  share: number;
}

export interface IData {
  name: string;
  symbol: string;
  uri: string;
  sellerFeeBasisPoints: number;
  creators: ICreator[] | undefined;
}

export class Metadata implements IMetadata {
  key!: number;
  updateAuthority!: PublicKey;
  mint!: PublicKey;
  data!: Data;
  constructor(fields: IMetadata) {
    Object.assign(this, fields);
  }
  static deserialize(data: Buffer): Metadata {
    return borsh.deserializeUnchecked(metadataSchema, Metadata, data, ExtendedReader);
  }
}

export class Data implements IData {
  name!: string;
  symbol!: string;
  uri!: string;
  sellerFeeBasisPoints!: number;
  creators: ICreator[] | undefined;
  constructor(fields: IData) {
    Object.assign(this, fields);
  }
}

export class Creator implements ICreator {
  address!: PublicKey;
  verified!: boolean;
  share!: number;
  constructor(fields: ICreator) {
    Object.assign(this, fields);
  }
}

const metadataSchema = new Map<Function, any>([
  [
    Metadata,
    {
      kind: 'struct',
      fields: [
        ['key', 'u8'],
        ['updateAuthority', 'publicKey'],
        ['mint', 'publicKey'],
        ['data', Data],
      ],
    },
  ],
  [
    Creator,
    {
      kind: 'struct',
      fields: [
        ['address', 'publicKey'],
        ['verified', 'bool'],
        ['share', 'u8'],
      ],
    },
  ],
  [
    Data,
    {
      kind: 'struct',
      fields: [
        ['name', 'string'],
        ['symbol', 'string'],
        ['uri', 'string'],
        ['sellerFeeBasisPoints', 'u16'],
        ['creators', [Creator]],
      ],
    },
  ],
]);
