import { field, map, variant, array } from '@race-foundation/borsh'
import { Ciphertext, Digest, Fields, Secret } from './types'
import { CiphertextAndDigest } from './events'

export interface SecretIdent {
    fromAddr: string
    toAddr: string | undefined
    randomId: number
    index: number
}

export abstract class RandomSpec {
    abstract asOptions(): string[]
}

@variant(0)
export class ShuffledList extends RandomSpec {
    @field(array('string'))
    options!: string[]
    constructor(fields: { options: string[] }) {
        super()
        Object.assign(this, fields)
    }
    asOptions(): string[] {
        return this.options
    }
}

@variant(1)
export class Lottery extends RandomSpec {
    @field(map('string', 'u16'))
    optionsAndWeights!: Map<string, number>
    constructor(fields: Fields<Lottery>) {
        super()
        Object.assign(this, fields)
    }
    asOptions(): string[] {
        const res: string[] = []
        for (const [k, v] of this.optionsAndWeights) {
            for (let i = 0; i < v; i++) {
                res.push(k)
            }
        }
        return res
    }
}

export class Lock {
    digest: Digest
    owner: string
    constructor(owner: string, digest: Digest) {
        this.digest = digest
        this.owner = owner
    }
}

export type MaskStatus = 'required' | 'applied' | 'removed'

export class Mask {
    status: MaskStatus
    readonly owner: string
    constructor(owner: string) {
        this.owner = owner
        this.status = 'required'
    }
}

export type CipherOwner =
    | {
          kind: 'unclaimed'
      }
    | {
          kind: 'assigned'
          addr: string
      }
    | {
          kind: 'multiAssigned'
          addrs: string[]
      }
    | {
          kind: 'revealed'
      }

export class LockedCiphertext {
    locks: Lock[]
    owner: CipherOwner
    ciphertext: Ciphertext
    constructor(text: Ciphertext) {
        this.ciphertext = text
        this.locks = []
        this.owner = { kind: 'unclaimed' }
    }
}

export class Share {
    fromAddr: string
    toAddr: string | undefined
    index: number
    secret: Secret | undefined
    constructor(fromAddr: string, index: number, toAddr?: string) {
        this.fromAddr = fromAddr
        this.index = index
        this.toAddr = toAddr
        this.secret = undefined
    }
}

export type RandomStatus =
    | {
          kind: 'ready'
      }
    | {
          kind: 'locking'
          addr: string
      }
    | {
          kind: 'masking'
          addr: string
      }
    | {
          kind: 'waiting-secrets'
      }
    | {
          kind: 'shared'
      }

export class RandomState {
    id: number
    size: number
    owners: string[]
    options: string[]
    status: RandomStatus
    masks: Mask[]
    ciphertexts: LockedCiphertext[]
    secretShares: Share[]
    revealed: Map<number, string>

    constructor(id: number, spec: RandomSpec, owners: string[]) {
        if (owners.length === 0) {
            throw new Error('No enough servers')
        }
        this.owners = owners
        const options = spec.asOptions()
        const ciphertexts = options.map(o => new LockedCiphertext(new TextEncoder().encode(o)))
        this.options = options
        this.size = options.length
        this.ciphertexts = ciphertexts
        this.masks = owners.map(o => new Mask(o))
        this.id = id
        this.revealed = new Map()
        this.secretShares = []
        this.status = { kind: 'masking', addr: owners[0] }
    }

    isFullyMasked(): boolean {
        return this.masks.every(m => m.status !== 'required')
    }

    isFullyLocked(): boolean {
        return this.masks.every(m => m.status === 'removed')
    }

    mask(addr: string, ciphertexts: Ciphertext[]) {
        if (this.status.kind === 'masking') {
            const a = this.status.addr
            if (a !== addr) {
                throw new Error('Invalid operator')
            }
            const mask = this.masks.find(m => m.owner === addr)
            if (mask === undefined) {
                throw new Error('Invalid operator')
            }
            if (mask.status !== 'required') {
                throw new Error('Duplicated mask')
            }
            if (ciphertexts.length !== this.ciphertexts.length) {
                throw new Error('Invalid ciphertexts')
            }
            for (let i = 0; i < this.ciphertexts.length; i++) {
                this.ciphertexts[i].ciphertext = ciphertexts[i]
            }
            mask.status = 'applied'
            this.updateStatus()
        } else {
            throw new Error('Invalid status:' + this.status.kind)
        }
    }

    lock(addr: string, ciphertextsAndDigests: CiphertextAndDigest[]) {
        if (this.status.kind === 'locking') {
            const a = this.status.addr
            if (a !== addr) {
                throw new Error('Invalid operator')
            }
            const mask = this.masks.find(m => m.owner === addr)
            if (mask === undefined) {
                throw new Error('Invalid operator')
            }
            if (ciphertextsAndDigests.length !== this.ciphertexts.length) {
                throw new Error('Invalid ciphertexts')
            }
            mask.status = 'removed'
            for (let i = 0; i < this.ciphertexts.length; i++) {
                const { ciphertext, digest } = ciphertextsAndDigests[i]
                this.ciphertexts[i].ciphertext = ciphertext
                this.ciphertexts[i].locks.push(new Lock(addr, digest))
            }
            this.updateStatus()
        } else {
            throw new Error('Invalid status:' + this.status.kind)
        }
    }

    assign(addr: string, indexes: number[]) {
        if (this.status.kind === 'ready' || this.status.kind === 'shared' || this.status.kind === 'waiting-secrets') {
            for (const idx of indexes) {
                let c = this.ciphertexts[idx]
                if (c.owner.kind === 'assigned' || c.owner.kind === 'revealed') {
                    throw new Error('Ciphertext already assigned')
                }
                c.owner = { kind: 'assigned', addr }
                let secrets = this.secretShares
                this.owners.forEach(o => {
                    secrets.push(new Share(o, idx, addr))
                })
            }

            this.status = { kind: 'waiting-secrets' }
        } else {
            throw new Error('Invalid status:' + this.status.kind)
        }
    }

    addSecretShare(share: Share) {
        const exist = this.secretShares.find(
            ss => ss.fromAddr === share.fromAddr && ss.toAddr === share.toAddr && ss.index === share.index
        )
        if (exist === undefined) {
            this.secretShares.push(share)
        }
    }

    reveal(indexes: number[]) {
        if (this.status.kind === 'ready' || this.status.kind === 'shared' || this.status.kind === 'waiting-secrets') {
            for (const idx of indexes) {
                let c = this.ciphertexts[idx]
                if (c.owner.kind !== 'revealed') {
                    c.owner = { kind: 'revealed' }
                    let secrets = this.secretShares
                    this.owners.forEach(o => {
                        secrets.push(new Share(o, idx))
                    })
                }
            }

            this.status = { kind: 'waiting-secrets' }
        } else {
            throw new Error('Invalid status:' + this.status.kind)
        }
    }

    listRequiredSecretsByFromAddr(fromAddr: string): SecretIdent[] {
        return this.secretShares
            .filter(ss => ss.secret === undefined && ss.fromAddr === fromAddr)
            .map(ss => ({
                fromAddr: ss.fromAddr,
                toAddr: ss.toAddr,
                randomId: this.id,
                index: ss.index,
            }))
    }

    listRevealedSecrets(): Map<number, Secret[]> {
        if (this.status.kind !== 'ready') {
            throw new Error('Secrets not ready, current status: ' + this.status.kind)
        }
        let res = new Map<number, Secret[]>()
        for (const ss of this.secretShares) {
            if (ss.toAddr === undefined) {
                let ciphertexts = res.get(ss.index)
                if (ciphertexts === undefined) {
                    res.set(ss.index, [ss.secret!])
                } else {
                    ciphertexts.push(ss.secret!)
                }
            }
        }
        return res
    }

    listAssignedCiphertexts(addr: string): Map<number, Ciphertext> {
        let res = new Map<number, Ciphertext>()
        for (let i = 0; i < this.ciphertexts.length; i++) {
            const c = this.ciphertexts[i]
            if (c.owner.kind === 'assigned' && c.owner.addr === addr) {
                res.set(i, c.ciphertext)
            }
        }
        return res
    }

    listRevealedCiphertexts(): Map<number, Ciphertext> {
        let res = new Map<number, Ciphertext>()
        for (let i = 0; i < this.ciphertexts.length; i++) {
            const c = this.ciphertexts[i]
            if (c.owner.kind === 'revealed') {
                res.set(i, c.ciphertext)
            }
        }
        return res
    }

    listSharedSecrets(toAddr: string): Map<number, Secret[]> {
        if (this.status.kind !== 'ready') {
            throw new Error('Secrets not ready, current status: ' + this.status.kind)
        }
        let res = new Map<number, Secret[]>()
        for (const ss of this.secretShares) {
            if (ss.toAddr === toAddr) {
                let secrets = res.get(ss.index)
                if (secrets === undefined) {
                    res.set(ss.index, [ss.secret!])
                } else {
                    secrets.push(ss.secret!)
                }
            }
        }
        return res
    }

    addRevealed(revealed: Map<number, string>) {
        for (const [index, value] of revealed) {
            if (index >= this.size) {
                throw new Error('Invalid index')
            }
            this.revealed.set(index, value)
        }
    }

    addSecret(fromAddr: string, toAddr: string | undefined, index: number, secret: Secret) {
        const secretShare = this.secretShares.find(
            ss => ss.fromAddr === fromAddr && ss.toAddr === toAddr && ss.index === index
        )
        if (secretShare !== undefined) {
            if (secretShare.secret === undefined) {
                const ciphertext = this.ciphertexts[secretShare.index]
                if (ciphertext !== undefined) {
                    secretShare.secret = secret
                } else {
                    throw new Error('Invalid secret')
                }
            } else {
                throw new Error('Duplicated secret')
            }
        }
        this.updateStatus()
    }

    listOperatingAddrs(): string[] {
        switch (this.status.kind) {
            case 'ready':
                return []
            case 'shared':
                return []
            case 'locking':
                return [this.status.addr]
            case 'masking':
                return [this.status.addr]
            case 'waiting-secrets':
                return this.secretShares.filter(s => s.secret === undefined).map(s => s.fromAddr)
        }
    }

    updateStatus() {
        if (this.status.kind === 'locking' && this.masks.every(m => m.status === 'removed')) {
            this.status = { kind: 'ready' }
            return
        }
        let mask = this.masks.find(m => m.status === 'required')
        if (mask !== undefined) {
            this.status = { kind: 'masking', addr: mask.owner }
            return
        }
        mask = this.masks.find(m => m.status === 'applied')
        if (mask !== undefined) {
            this.status = { kind: 'locking', addr: mask.owner }
            return
        }
        if (this.secretShares.find(s => s.secret === undefined) !== undefined) {
            this.status = { kind: 'waiting-secrets' }
            return
        }
        this.status = { kind: 'shared' }
    }
}
