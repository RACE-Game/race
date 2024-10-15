import { deserialize, field, map, struct } from '@race-foundation/borsh';
import { sha256 } from './encryptor';

export class CheckpointOnChain {
  @field('u8-array')
  root!: Uint8Array

  @field('usize')
  size!: number

  @field('u64')
  accessVersion!: bigint

  constructor(fields: any) {
    Object.assign(this, fields)
  }

  static fromRaw(raw: Uint8Array): CheckpointOnChain {
    return deserialize(CheckpointOnChain, raw);
  }
}

export class VersionedData {
  @field('usize')
  id!: number;

  @field('u64')
  version!: bigint;

  @field('string')
  sha!: string;

  @field('u8-array')
  data!: Uint8Array;
  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

export class CheckpointOffChain {
  @field(map('usize', struct(VersionedData)))
  data!: Map<number, VersionedData>

  @field(map('usize', 'u8-array'))
  proofs!: Map<number, Uint8Array>

  constructor(fields: any) {
    Object.assign(this, fields);
  }
}

/// Represent the on-chain checkpoint.
export class Checkpoint {
  @field('u8-array')
  root!: Uint8Array;

  @field('u64')
  accessVersion!: bigint;

  @field(map('usize', struct(VersionedData)))
  data!: Map<number, VersionedData>;

  @field(map('usize', 'u8-array'))
  proofs!: Map<number, Uint8Array>;

  constructor(fields: any) {
    Object.assign(this, fields)
  }

  static default(): Checkpoint {
    return new Checkpoint({ accessVersion: 0n, data: new Map() })
  }

  static fromParts(offchainPart: CheckpointOffChain, onchainPart: CheckpointOnChain): Checkpoint {
    let checkpoint = Checkpoint.default();
    checkpoint.proofs = offchainPart.proofs;
    checkpoint.data = offchainPart.data;
    checkpoint.accessVersion = onchainPart.accessVersion;
    checkpoint.root = onchainPart.root;
    return checkpoint;
  }

  static fromRaw(raw: Uint8Array): Checkpoint {
    if (raw.length === 0) {
      return Checkpoint.default();
    }
    return deserialize(Checkpoint, raw);
  }

  static fromData(id: number, version: bigint, data: Uint8Array): Checkpoint {
    return new Checkpoint({
      data: new Map([[id, new VersionedData({
        version, data
      })]])
    });
  }

  clone(): Checkpoint {
    return new Checkpoint({
      accessVersion: this.accessVersion,
      data: new Map(this.data.entries())
    })
  }

  getData(id: number): Uint8Array | undefined {
    return this.data.get(id)?.data;
  }

  getSha(id: number): string | undefined {
    return this.data.get(id)?.sha;
  }

  setVersion(id: number, version: bigint) {
    const data = this.data.get(id);
    if (data !== undefined) {
      data.version = version;
    }
  }

  setSha(id: number, sha: string) {
    const data = this.data.get(id);
    if (data !== undefined) {
      data.sha = sha;
    }
  }

  async setData(id: number, data: Uint8Array) {
    const sha = await sha256(data);
    const old = this.data.get(id);
    if (old !== undefined) {
      old.data = data;
      old.version += 1n;
      old.sha = sha;
    }
    this.updateRootAndProofs();
  }

  updateRootAndProofs() {

  }

  maybeInitData(id: number, data: Uint8Array) {
    if (!this.data.has(id)) {
      this.data.set(id, new VersionedData({
        version: 0n,
        data
      }))
    }
  }

  setAccessVersion(accessVersion: bigint) {
    this.accessVersion = accessVersion;
  }

  __setVersion(id: number, version: bigint) {
    let vd = this.data.get(id);
    if (vd !== undefined) {
      vd.version = version;
    }
  }

  getVersion(id: number): bigint {
    return this.data.get(id)?.version || 0n
  }
}
