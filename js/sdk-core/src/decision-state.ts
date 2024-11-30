import { Ciphertext, Digest, Secret } from './types'

export type DecisionStatus = 'asked' | 'answered' | 'releasing' | 'released'

export class Answer {
    digest: Digest
    ciphertext: Ciphertext
    constructor(ciphertext: Ciphertext, digest: Digest) {
        this.digest = digest
        this.ciphertext = ciphertext
    }
}

export class DecisionState {
    id: number
    owner: string
    status: DecisionStatus
    answer: Answer | undefined
    secret: Secret | undefined
    value: string | undefined
    constructor(id: number, owner: string) {
        this.id = id
        this.owner = owner
        this.status = 'asked'
        this.secret = undefined
        this.answer = undefined
        this.value = undefined
    }

    setAnswer(owner: string, ciphertext: Ciphertext, digest: Digest) {
        if (this.owner !== owner) {
            throw new Error('Invalid decision owner')
        }
        if (this.status !== 'asked') {
            throw new Error('Invalid decision status')
        }
        this.answer = new Answer(ciphertext, digest)
        this.status = 'answered'
    }

    release() {
        if (this.status !== 'answered') {
            throw new Error('Invalid decision status')
        } else {
            this.status = 'releasing'
        }
    }

    addReleased(value: string) {
        if (this.status !== 'released') {
            throw new Error('Invalid decision status')
        } else {
            this.value = value
        }
    }

    addSecret(owner: string, secret: Secret) {
        if (this.status !== 'releasing') {
            throw new Error('Invalid decision status')
        } else if (this.owner !== owner) {
            throw new Error('Invalid decision owner')
        } else {
            this.secret = secret
            this.status = 'released'
        }
    }
}
