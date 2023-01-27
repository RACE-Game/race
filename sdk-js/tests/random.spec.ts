import { assert } from 'chai'
import { RandomSpec, ShuffledList } from '../src/random'
import { RandomState } from '../src/RandomState'
import { Ciphertext, SecretDigest } from '../src/types/common'

describe('RandomState', () => {

  const MASK_ARG = [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]
  const LOCK_ARG: Array<[Ciphertext, SecretDigest]> =
    [[Uint8Array.of(1), Uint8Array.of(1)],
    [Uint8Array.of(2), Uint8Array.of(2)],
    [Uint8Array.of(3), Uint8Array.of(3)]]

  it('create new random state', () => {
    const rnd = new ShuffledList(['a', 'b', 'c'])
    const state = new RandomState(0, rnd, ['alice', 'bob', 'charlie'])
    assert.lengthOf(state._masks, 3)
  })

  it("mask", () => {
    const rnd = new ShuffledList(['a', 'b', 'c'])
    const state = new RandomState(0, rnd, ['alice', 'bob'])
    assert.deepEqual(state._status, { status: 'masking', addr: 'alice' })
    state.mask('alice', MASK_ARG)
    assert.deepEqual(state._status, { status: 'masking', addr: 'bob' })
    assert.equal(state.isFullyMasked(), false)
    state.mask('bob', MASK_ARG)
    assert.deepEqual(state._status, { status: 'locking', addr: 'alice' })
    assert.equal(state.isFullyMasked(), true)
  })

  it("lock", () => {
    const rnd = new ShuffledList(['a', 'b', 'c'])
    const state = new RandomState(0, rnd, ['alice', 'bob'])
    assert.throw(() => {
      state.lock('alice', LOCK_ARG)
    }, "Invalid cipher status")

    state.mask('alice', MASK_ARG)
    state.mask('bob', MASK_ARG)
    assert.deepEqual(state._status, { status: 'locking', addr: 'alice' })
    state.lock('alice', LOCK_ARG)
    assert.deepEqual(state._status, { status: 'locking', addr: 'bob' })
    state.lock('bob', LOCK_ARG)
    assert.equal(state.isFullyLocked(), true)
    assert.equal(state._status.status, "ready")
  })
})
