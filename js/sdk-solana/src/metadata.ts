import { PublicKey } from '@solana/web3.js';
import { publicKeyExt } from './utils';
import { deserialize, field, array, struct, option } from '@race/borsh';

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


export class Creator implements ICreator {
  @field(publicKeyExt)
  address!: PublicKey;
  @field('bool')
  verified!: boolean;
  @field('u8')
  share!: number;
  constructor(fields: ICreator) {
    Object.assign(this, fields);
  }
}


export class Data implements IData {
  @field('string')
  name!: string;
  @field('string')
  symbol!: string;
  @field('string')
  uri!: string;
  @field('u16')
  sellerFeeBasisPoints!: number;
  @field(option(array(struct(Creator))))
  creators: ICreator[] | undefined;
  constructor(fields: IData) {
    Object.assign(this, fields);
  }
}


export class Metadata implements IMetadata {
  @field('u8')
  key!: number;
  @field(publicKeyExt)
  updateAuthority!: PublicKey;
  @field(publicKeyExt)
  mint!: PublicKey;
  @field(struct(Data))
  data!: Data;
  constructor(fields: IMetadata) {
    Object.assign(this, fields);
  }
  static deserialize(data: Buffer): Metadata {
    return deserialize(Metadata, new Uint8Array(data.buffer));
  }
}
