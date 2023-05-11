import { field, map, variant, vec } from '@race/borsh';
import { Ciphertext, Digest, Fields, Secret } from './types';
import { CiphertextAndDigest } from './events';

const textEncoder = new TextEncoder();

export interface SecretIdent {
  fromAddr: string
  toAddr: string | undefined
  randomId: bigint
  index: number
}

export abstract class RandomSpec {
  abstract asOptions(): string[];
}

@variant(0)
export class ShuffledList extends RandomSpec {
  @field(vec('string'))
  options!: string[];
  constructor(fields: Fields<ShuffledList>) {
    super();
    Object.assign(this, fields);
  }
  asOptions(): string[] {
    return this.options;
  }
}

@variant(1)
export class Lottery extends RandomSpec {
  @field(map('string', 'u16'))
  optionsAndWeights!: Map<string, number>
  constructor(fields: Fields<Lottery>) {
    super();
    Object.assign(this, fields);
  }
  asOptions(): string[] {
    const res: string[] = [];
    for (const [k, v] of this.optionsAndWeights) {
      for (let i = 0; i < v; i++) {
        res.push(k);
      }
    }
    return res;
  }
}

export class ILock {
  digest: Digest;
  owner: string;
  constructor(owner: string, digest: Digest) {
    this.digest = digest;
    this.owner = owner;
  }
}

export type MaskStatus = 'required' | 'applied' | 'removed';

export class Mask {
  status: MaskStatus;
  readonly owner: string;
  constructor(owner: string) {
    this.owner = owner;
    this.status = 'required';
  }
}

export type CipherOwner = {
  kind: 'unclaimed'
} | {
  kind: 'alligned',
  addr: string;
} | {
  kind: 'multiAssigned',
  addrs: string[];
} | {
  kind: 'revealed'
};

export class LockedCiphertext {
  locks: Lock[];
  owner: CipherOwner;
  ciphertext: Ciphertext;
  constructor(text: Ciphertext) {
    this.ciphertext = text;
    this.locks = [];
    this.owner = { kind: 'unclaimed' };
  }
}

export class Share {
  fromAddr: string;
  toAddr: string | undefined;
  index: number;
  secret: Secret | undefined;
  constructor(fromAddr: string, index: number, toAddr?: string) {
    this.fromAddr = fromAddr;
    this.index = index;
    this.toAddr = toAddr;
    this.secret = undefined;
  }
}

export type RandomStatus = {
  kind: 'ready'
} | {
  kind: 'locking',
  addr: string
} | {
  kind: 'masking',
  addr: string
} | {
  kind: 'waiting-secrets'
};

export class RandomState {
  id: bigint;
  size: number;
  owners: string[];
  options: string[];
  status: RandomStatus;
  masks: Mask[];
  ciphertexts: LockedCiphertext[];
  secret_shares: Share[];
  revealed: Map<number, string>;

  constructor(id: bigint, spec: RandomSpec, owners: string[]) {
    if (owners.length === 0) {
      throw new Error('No enough servers');
    }
    this.owners = owners;
    this.options = spec.asOptions();
    this.size = this.options.length;
    this.ciphertexts = this.options.map(o => new LockedCiphertext(textEncoder.encode(o)));
    this.masks = owners.map(o => new Mask(o));
    this.id = id;
    this.revealed = new Map();
    this.secret_shares = [];
    this.status = { kind: 'masking', addr: owners[0] };
  }

  mask(addr: string, ciphertexts: Ciphertext[]) {

  }

  lock(addr: string, ciphertextsAndDigests: CiphertextAndDigest[]) {

  }

  assign(addr: string, indexes: number[]) {

  }

  addSecretShare(share: Share) {

  }

  reveal(indexes: number[]) {

  }

  listRequiredSecretsByFromAddr(fromAddr: string): SecretIdent[] {
    return [];
  }

  listRevealedSecrets(): Map<number, Ciphertext[]> {
    return new Map();

  }

  listAssignedCiphertexts(addr: string): Map<number, Ciphertext> {
    return new Map();

  }

  listRevealedCiphertexts(): Map<number, Ciphertext> {
    return new Map();
  }

  listSharedSecrets(): Map<number, Secret[]> {
    return new Map();
  }

  addRevealed(revealed: Map<number, string>) {

  }

  addSecret(fromAddr: string, toAddr: string | undefined, index: number, secret: Secret) {

  }

  listOperatingAddrs(): string[] {
    return [];
  }

  updateStatus() {

  }
}
