import { deserialize, field, map, struct } from '@race-foundation/borsh';

export class VersionedData {
  @field('u64')
  version!: bigint;
  @field('u8-array')
  data!: Uint8Array;
  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

/// Represent the on-chain checkpoint.
export class Checkpoint {
  @field(map('usize', struct(VersionedData)))
  data!: Map<number, VersionedData>;
  constructor(fields: any) {
    Object.assign(this, fields)
  }

  static default(): Checkpoint {
    return new Checkpoint({ data: new Map() })
  }

  static fromRaw(raw: Uint8Array): Checkpoint {
    return deserialize(Checkpoint, raw);
  }

  static fromData(id: number, version: bigint, data: Uint8Array): Checkpoint {
    return new Checkpoint({
      data: new Map([[id, new VersionedData({
        version, data
      })]])
    });
  }

  getData(id: number): Uint8Array | undefined {
    return this.data.get(id)?.data
  }

  setData(id: number, data: Uint8Array) {
    const ver = this.getVersion(0);
    let vd = this.data.get(id);
    if (vd !== undefined) {
      vd.data = data;
      vd.version = ver + 1n;
    } else {
      this.data.set(id, new VersionedData({
        version: ver + 1n,
        data
      }))
    }
    this.__setVersion(0, ver + 1n);
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
