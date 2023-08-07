import { CiphertextAndDigest } from '../src/events';
import { RandomState, ShuffledList, Share } from '../src/random-state';
import { assert } from 'chai';

describe('Test RandomSpec', () => {
    it('new RandomSpec', () => {
        const random = new ShuffledList({ options: ['a', 'b', 'c'] });
        const state = new RandomState(0, random, ['alice', 'bob', 'charlie']);
        assert.equal(3, state.masks.length);
    });
});

describe('Test RandomState', () => {
    it('mask', () => {
        const random = new ShuffledList({ options: ['a', 'b', 'c'] });
        const state = new RandomState(0, random, ['alice', 'bob']);

        assert.deepEqual(state.status, { kind: 'masking', addr: 'alice' });
        state.mask('alice', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        assert.deepEqual(state.status, { kind: 'masking', addr: 'bob' });
        assert.equal(state.isFullyMasked(), false);
        state.mask('bob', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        assert.deepEqual(state.status, { kind: 'locking', addr: 'alice' });
        assert.equal(state.isFullyMasked(), true);
    });

    it('addSecretShare', () => {
        const random = new ShuffledList({ options: ['a', 'b', 'c'] });
        const state = new RandomState(0, random, ['alice', 'bob']);
        const share1 = new Share('alice', 0, undefined);
        const share2 = new Share('alice', 0, undefined);
        state.addSecretShare(share1);
        state.addSecretShare(share2);
        assert.equal(state.secretShares.length, 1);
    });

    it('lock', () => {
        const random = new ShuffledList({ options: ['a', 'b', 'c'] });
        const state = new RandomState(0, random, ['alice', 'bob']);

        state.mask('alice', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        state.mask('bob', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        state.lock('alice', [
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(1), digest: Uint8Array.of(1) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(2), digest: Uint8Array.of(2) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(3), digest: Uint8Array.of(3) }),
        ]);
        assert.deepEqual(state.status, { kind: 'locking', addr: 'bob' });
        assert.equal(state.isFullyLocked(), false);
        state.lock('bob', [
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(1), digest: Uint8Array.of(1) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(2), digest: Uint8Array.of(2) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(3), digest: Uint8Array.of(3) }),
        ]);
        assert.deepEqual(state.status, { kind: 'ready' });
        assert.equal(state.isFullyLocked(), true);
    });

    it('listRequiredSecrets', () => {
        const random = new ShuffledList({ options: ['a', 'b', 'c'] });
        const state = new RandomState(0, random, ['alice', 'bob']);
        state.mask('alice', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        state.mask('bob', [Uint8Array.of(1), Uint8Array.of(2), Uint8Array.of(3)]);
        state.lock('alice', [
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(1), digest: Uint8Array.of(1) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(2), digest: Uint8Array.of(2) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(3), digest: Uint8Array.of(3) }),
        ]);
        state.lock('bob', [
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(1), digest: Uint8Array.of(1) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(2), digest: Uint8Array.of(2) }),
            new CiphertextAndDigest({ ciphertext: Uint8Array.of(3), digest: Uint8Array.of(3) }),
        ]);
        assert.equal(state.status.kind, 'ready');
        state.reveal([0]);
        assert.equal(state.listRequiredSecretsByFromAddr('alice').length, 1);
        assert.equal(state.listRequiredSecretsByFromAddr('bob').length, 1);
        state.addSecret('alice', undefined, 0, Uint8Array.of(1, 2, 3));
        state.reveal([0]);
        assert.equal(state.listRequiredSecretsByFromAddr('alice').length, 0);
        assert.equal(state.listRequiredSecretsByFromAddr('bob').length, 1);
        state.addSecret('bob', undefined, 0, Uint8Array.of(1, 2, 3));
        assert.equal(state.listRequiredSecretsByFromAddr('alice').length, 0);
        assert.equal(state.listRequiredSecretsByFromAddr('bob').length, 0);
        assert.equal(state.status.kind, 'shared');
    });
});
