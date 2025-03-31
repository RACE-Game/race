import { array, deserialize, field, map, option, struct } from '@race-foundation/borsh'
import { sha256 } from './encryptor'
import { Fields } from './types'

export class Versions {
    @field('u64')
    accessVersion!: bigint

    @field('u64')
    settleVersion!: bigint

    static default(): Versions {
        return new Versions({ accessVersion: 0n, settleVersion: 0n })
    }

    constructor(fields: Fields<Versions>) {
        Object.assign(this, fields)
    }
}

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
        return deserialize(CheckpointOnChain, raw)
    }
}

export class GameSpec {
    @field('string')
    readonly gameAddr!: string

    @field('usize')
    readonly gameId!: number

    @field('string')
    readonly bundleAddr!: string

    @field('u16')
    readonly maxPlayers!: number

    constructor(fields: Fields<GameSpec>) {
        Object.assign(this, fields)
    }
}

export class VersionedData {
    @field('usize')
    id!: number

    @field(struct(Versions))
    versions!: Versions

    @field('u8-array')
    data!: Uint8Array

    @field('u8-array')
    sha!: Uint8Array

    @field(struct(GameSpec))
    spec!: GameSpec

    constructor(fields: any) {
        Object.assign(this, fields)
    }
}

export class CheckpointOffChain {
    @field(map('usize', struct(VersionedData)))
    data!: Map<number, VersionedData>

    @field(map('usize', 'u8-array'))
    proofs!: Map<number, Uint8Array>

    constructor(fields: any) {
        Object.assign(this, fields)
    }

    static deserialize(raw: Uint8Array): CheckpointOffChain {
        return deserialize(CheckpointOffChain, raw)
    }
}

export class CheckpointOffChainList {

    @field(array(option(struct(CheckpointOffChain))))
    checkpoints!: (CheckpointOffChain | undefined)[]

    constructor(fields: any) {
        Object.assign(this, fields)
    }

    static deserialize(raw: Uint8Array): CheckpointOffChainList {
        return deserialize(CheckpointOffChainList, raw)
    }
}

/// Represent the on-chain checkpoint.
export class Checkpoint {
    @field('u8-array')
    root!: Uint8Array

    @field('u64')
    accessVersion!: bigint

    @field(map('usize', struct(VersionedData)))
    data!: Map<number, VersionedData>

    @field(map('usize', 'u8-array'))
    proofs!: Map<number, Uint8Array>

    constructor(fields: any) {
        Object.assign(this, fields)
    }

    static default(): Checkpoint {
        return new Checkpoint({ accessVersion: 0n, data: new Map() })
    }

    static fromParts(offchainPart: CheckpointOffChain, onchainPart: CheckpointOnChain): Checkpoint {
        let checkpoint = Checkpoint.default()
        checkpoint.proofs = offchainPart.proofs
        checkpoint.data = offchainPart.data
        checkpoint.accessVersion = onchainPart.accessVersion
        checkpoint.root = onchainPart.root
        return checkpoint
    }

    static fromRaw(raw: Uint8Array): Checkpoint {
        if (raw.length === 0) {
            return Checkpoint.default()
        }
        return deserialize(Checkpoint, raw)
    }

    static fromData(id: number, version: bigint, data: Uint8Array): Checkpoint {
        return new Checkpoint({
            data: new Map([
                [
                    id,
                    new VersionedData({
                        version,
                        data,
                    }),
                ],
            ]),
        })
    }

    clone(): Checkpoint {
        return new Checkpoint({
            accessVersion: this.accessVersion,
            data: new Map(this.data.entries()),
        })
    }

    getData(id: number): Uint8Array | undefined {
        return this.data.get(id)?.data
    }

    getSha(id: number): Uint8Array | undefined {
        return this.data.get(id)?.sha
    }

    setAccessVersion(accessVersion: bigint) {
        this.accessVersion = accessVersion
    }

    async initData(id: number, data: Uint8Array, spec: GameSpec) {
        if (this.data.has(id)) {
            throw new Error(`Checkpoint ${id} already exists`)
        }
        const sha = await sha256(data)
        const versionedData = new VersionedData({
            id,
            data,
            spec,
            sha,
            versions: Versions.default(),
        })
        this.data.set(id, versionedData)
        this.updateRootAndProofs()
    }

    async initVersionedData(versionedData: VersionedData) {
        this.data.set(versionedData.id, versionedData)
    }

    async setData(id: number, data: Uint8Array) {
        const sha = await sha256(data)
        const old = this.data.get(id)
        if (old !== undefined) {
            old.data = data
            old.versions.settleVersion += 1n
            old.sha = sha
        } else {
            throw new Error(`Checkpoint ${id} is missing`)
        }
        this.updateRootAndProofs()
    }

    getVersionedData(id: number): VersionedData | undefined {
        return this.data.get(id)
    }

    containsVersionedData(id: number): boolean {
        return this.data.has(id)
    }

    updateRootAndProofs() {}
}
