import { Address, Ciphertext, RandomId, SecretDigest, SecretIdent, SecretKey } from './types/common'
import { RandomStatus, Mask, LockedCiphertext, SecretShare, RandomSpec } from './random'

export class RandomState {
  readonly _id: RandomId
  readonly _size: number
  _owners: Address[]
  _options: string[]
  _status: RandomStatus
  _masks: Mask[]
  _ciphertexts: LockedCiphertext[]
  _secretShares: SecretShare[]
  _revealed: Record<string, string>

  constructor (id: number, rnd: RandomSpec, owners: string[]) {
    const firstOwner = owners[0]
    if (firstOwner === undefined) {
      throw new Error('Empty owners')
    }
    this._id = id
    this._options = rnd.options()
    this._size = rnd.size()
    this._owners = owners
    this._status = { status: 'masking', addr: firstOwner }
    this._masks = owners.map((owner) => {
      return { status: 'required', owner }
    })
    this._ciphertexts = rnd.options().map(opt => {
      return new LockedCiphertext(new TextEncoder().encode(opt))
    })
    this._secretShares = []
    this._revealed = {}
  }

  isFullyMasked (): boolean {
    return this._masks.every(m => m.status !== 'required')
  }

  isFullyLocked (): boolean {
    return this._masks.every(m => m.status === 'removed')
  }

  public mask (addr: Address, ciphertexts: Ciphertext[]) {
    if (this._status.status !== 'masking') {
      throw new Error('Invalid cipher status')
    }
    const maskAddr = this._status.addr
    if (maskAddr !== addr) {
      throw new Error('Invalid mask provider')
    }
    const mask = this._masks.find(m => m.owner == addr)
    if (mask == null) {
      throw new Error('Invalid operator')
    }
    if (mask.status !== 'required') {
      throw new Error('Duplicated mask')
    }
    if (ciphertexts.length != this._ciphertexts.length) {
      throw new Error('Invalid ciphertexts')
    }
    this._ciphertexts.forEach((c, i) => {
      const ciphertext = ciphertexts[i]
      if (ciphertext == null) {
        throw new Error('Invalid ciphertexts')
      } else {
        c.ciphertext = ciphertext
      }
    })
    mask.status = 'applied'
    const nextMask = this._masks.find(m => m.status === 'required')
    if (nextMask != null) {
      this._status = { status: 'masking', addr: nextMask.owner }
    } else {
      this._status = { status: 'locking', addr: this._masks[0]!.owner }
    }
  }

  public lock (addr: Address, ciphertextsAndDigests: Array<[Ciphertext, SecretDigest]>) {
    if (this._status.status != 'locking') {
      throw new Error('Invalid cipher status')
    }
    const lockAddr = this._status.addr
    if (lockAddr !== addr) {
      throw new Error('Invalid lock provider')
    }
    const mask = this._masks.find(m => m.owner === addr)
    if (mask == null) {
      throw new Error('Invalid operator')
    }
    if (mask.status !== 'applied') {
      throw new Error('Duplicated lock')
    }
    if (this._ciphertexts.length !== ciphertextsAndDigests.length) {
      throw new Error('Invalid ciphertexts length')
    }
    mask.status = 'removed'
    this._ciphertexts.forEach((c, i) => {
      const item = ciphertextsAndDigests[i]
      if (item == null) {
        throw new Error('Invalid ciphertexts')
      } else {
        const [ciphertext, digest] = item
        c.ciphertext = ciphertext
        c.locks.push({ digest, owner: addr })
      }
    })
    const nextMask = this._masks.find(m => m.status === 'applied')
    if (nextMask != null) {
      this._status = { status: 'locking', addr: nextMask.owner }
    } else {
      this._status = { status: 'ready' }
    }
  }

  public assign (addr: Address, indexes: number[]) {
    if (this._status.status !== 'ready' && this._status.status !== 'waiting-secrets') {
      throw new Error('Invalid cipher status')
    }
    const duplicated = indexes
      .map(idx => this._ciphertexts[idx])
      .filter((c): c is LockedCiphertext => {
        return !(c == null) && ['assigned', 'revealed'].includes(c.owner.type)
      })
    if (duplicated.length > 0) {
      throw new Error('Ciphertext already assigned')
    }
    for (const i of indexes) {
      const lockedCiphertext = this._ciphertexts[i]
      if (lockedCiphertext != null) {
        lockedCiphertext.owner = { type: 'assigned', addr }
      }
      for (const o of this._owners) {
        this._secretShares.push({
          fromAddr: o,
          toAddr: addr,
          index: i,
          secret: null
        })
      }
    }
    this._status = { status: 'waiting-secrets' }
  }

  public reveal (indexes: number[]) {
    if (this._status.status !== 'ready' && this._status.status !== 'waiting-secrets') {
      throw new Error('Invalid cipher status')
    }
    const duplicated = indexes
      .map(idx => this._ciphertexts[idx])
      .filter((c): c is LockedCiphertext => {
        return !(c == null) && ['revealed'].includes(c.owner.type)
      })
    if (duplicated.length > 0) {
      throw new Error('Ciphertext already assigned')
    }
    for (const i of indexes) {
      const lockedCiphertext = this._ciphertexts[i]
      if (lockedCiphertext != null) {
        lockedCiphertext.owner = { type: 'revealed' }
      }
      for (const o of this._owners) {
        this._secretShares.push({
          fromAddr: o,
          toAddr: null,
          index: i,
          secret: null
        })
      }
    }
    this._status = { status: 'waiting-secrets' }
  }

  public listRequiredSecretsByFrom (fromAddr: Address): SecretIdent[] {
    return this._secretShares
      .filter((s) => s.secret == null && s.fromAddr === fromAddr)
      .map((s) => ({
        fromAddr: s.fromAddr,
        toAddr: s.toAddr,
        randomId: this._id,
        index: s.index
      }))
  }

  public listRevealedSecrets (): Record<number, Ciphertext[]> {
    if (this._status.status !== 'ready') {
      throw new Error('Secrets not ready')
    }
    return this._secretShares
      .filter((s) => s.toAddr === null)
      .reduce<Record<number, Ciphertext[]>>((acc, s) => {
      if (s.secret != null) {
        const secrets = acc[s.index]
        if (secrets == null) {
          acc[s.index] = [s.secret]
        } else {
          secrets.push(s.secret)
        }
      }
      return acc
    }, {})
  }

  public listAssignedCiphertexts (addr: Address): Record<number, Ciphertext> {
    return this._ciphertexts
      .reduce<Record<number, Ciphertext>>((acc, c, i) => {
      if (c.owner.type === 'assigned' && c.owner.addr === addr) {
        acc[i] = c.ciphertext
      }
      return acc
    }, {})
  }

  public listRevealedCiphertexts (): Record<number, Ciphertext> {
    return this._ciphertexts
      .reduce<Record<number, Ciphertext>>((acc, c, i) => {
      if (c.owner.type === 'revealed') {
        acc[i] = c.ciphertext
      }
      return acc
    }, {})
  }

  public listSharedSecrets (toAddr: Address): Record<number, SecretKey[]> {
    if (this._status.status === 'ready') {
      throw new Error('Secrets not ready')
    }
    return this._secretShares
      .reduce<Record<number, SecretKey[]>>((acc, s) => {
      if (s.toAddr === toAddr) {
        const secrets = acc[s.index]
        if (secrets == null) {
          acc[s.index] = [s.secret!]
        } else {
          secrets.push(s.secret!)
        }
      }
      return acc
    }, {})
  }

  public addRevealed (revealed: Record<string, string>) {
    for (const [key, value] of Object.entries(revealed)) {
      const index = Number(key)
      if (index >= this._size) {
        throw new Error('Invalid index')
      }
      this._revealed[index] = value
    }
  }

  public get revealed () {
    return this._revealed
  }

  public addSecret (fromAddr: Address, toAddr: string | null, index: number, secret: SecretKey) {
    const secretShare = this._secretShares.find(s => {
      s.fromAddr === fromAddr && s.toAddr === toAddr && s.index === index
    })
    if (secretShare != null) {
      if (secretShare.secret != null) {
        throw new Error('Duplicated secret')
      } else if (this._ciphertexts[secretShare.index] == null) {
        throw new Error('Invalid secret')
      } else {
        secretShare.secret = secret
      }
    }

    if (this._secretShares.every(s => s.secret)) {
      this._status = { status: 'ready' }
    }
  }
}
