import { deserialize, field, map, struct } from '@race-foundation/borsh';

export class VersionedData {
  @field('u64')
  version!: bigint;

  @field('string')
  sha!: string;

  @field('u8-array')
  data!: Uint8Array;
  constructor(fields: any) {
    Object.assign(this, fields)
  }
}

/// Represent the on-chain checkpoint.
export class Checkpoint {
  @field('u64')
  accessVersion!: bigint;

  @field(map('usize', struct(VersionedData)))
  data!: Map<number, VersionedData>;
  constructor(fields: any) {
    Object.assign(this, fields)
  }

  static default(): Checkpoint {
    return new Checkpoint({ accessVersion: 0n, data: new Map() })
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

  setData(id: number, data: Uint8Array, sha: string, version: bigint) {
    this.data.set(id, new VersionedData({
      version, sha, data
    }));
  }

  // initData(id: number, data: Uint8Array) {
  //   const version = this.getVersion(0);
  //   this.data.set(id, new VersionedData({
  //     version,
  //     data
  //   }));
  // }

  maybeInitData(id: number, data: Uint8Array) {
    if (!this.data.has(id)) {
      this.data.set(id, new VersionedData({
        version: 0n,
        data
      }))
    }
  }

  // setData(id: number, data: Uint8Array) {
  //   let vd = this.data.get(id);
  //   if (vd !== undefined) {
  //     vd.data = data;
  //     vd.version += 1n;
  //   }
  // }

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
