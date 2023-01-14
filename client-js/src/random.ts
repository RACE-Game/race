import { Address, Ciphertext, RandomId, SecretDigest, SecretIdent, SecretKey } from "./types/common";

export interface RandomSpec {
    options(): string[]
    size(): number
}

export class ShuffledList implements RandomSpec {
    readonly _options: string[]

    constructor(options: string[]) {
        this._options = options;
    }

    options(): string[] {
        return this._options;
    }

    size(): number {
        return this._options.length;
    }
}

export type MaskStatus = "required" | "applied" | "removed";

export type Mask = {
    status: MaskStatus,
    owner: Address
};

export type Lock = {
    digest: SecretDigest,
    owner: Address
}

export type CipherOwner =
    { type: "unclaimed" }
    | { type: "assigned", addr: Address }
    | { type: "multi_assigned", addrs: Address[] }
    | { type: "revealed" };

export class LockedCiphertext {
    locks: Lock[]
    owner: CipherOwner
    ciphertext: Ciphertext

    constructor(ciphertext: Ciphertext) {
        this.locks = [];
        this.owner = { type: "unclaimed" };
        this.ciphertext = ciphertext;
    }
}

export type SecretShare = {
    fromAddr: Address,
    toAddr: Address | null,
    index: number,
    secret: SecretKey | null
}

export type RandomStatus =
    { status: "ready" }
    | { status: "locking", addr: Address }
    | { status: "masking", addr: Address }
    | { status: "waiting_secrets" };

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

    constructor(id: string, rnd: RandomSpec, owners: string[]) {
        const firstOwner = owners[0];
        if (!firstOwner) {
            throw new Error("Empty owners");
        }
        this._id = id;
        this._options = rnd.options();
        this._size = rnd.size();
        this._owners = owners;
        this._status = { status: "masking", addr: firstOwner }
        this._masks = owners.map((owner) => {
            return { status: "required", owner }
        });
        this._ciphertexts = rnd.options().map(opt => {
            return new LockedCiphertext(new TextEncoder().encode(opt))
        })
        this._secretShares = [];
        this._revealed = {};
    }

    isFullyMasked(): boolean {
        return this._masks.every(m => m.status == "required")
    }

    isFullyLocked(): boolean {
        return this._masks.every(m => m.status == "removed")
    }

    public mask(addr: Address, ciphertexts: Ciphertext[]) {
        if (this._status.status !== "masking") {
            throw new Error("Invalid cipher status");
        }
        const maskAddr = this._status.addr;
        if (maskAddr !== addr) {
            throw new Error("Invalid mask provider");
        }
        let mask = this._masks.find(m => m.owner == addr);
        if (!mask) {
            throw new Error("Invalid operator");
        }
        if (mask.status !== "required") {
            throw new Error("Duplicated mask");
        }
        if (ciphertexts.length != this._ciphertexts.length) {
            throw new Error("Invalid ciphertexts");
        }
        for (let i = 0; i < this._ciphertexts.length; i++) {
            let lockedCiphertext = this._ciphertexts[i]!;
            lockedCiphertext.ciphertext = ciphertexts[i]!;
        }
        mask.status = "applied";
        let nextMask = this._masks.find(m => m.status === "required");
        if (nextMask) {
            this._status = { status: "masking", addr: nextMask.owner };
        } else {
            this._status = { status: "locking", addr: this._masks[0]!.owner };
        }
    }

    public lock(addr: Address, ciphertextsAndTests: Array<[Ciphertext, SecretDigest]>) {
        if (this._status.status != "locking") {
            throw new Error("Invalid cipher status");
        }
        const lockAddr = this._status.addr;
        if (lockAddr !== addr) {
            throw new Error("Invalid lock provider");
        }
        let mask = this._masks.find(m => m.owner !== addr);
        if (!mask) {
            throw new Error("Invalid operator");
        }
        if (mask.status !== "applied") {
            throw new Error("Duplicated lock");
        }
        if (this._ciphertexts.length !== ciphertextsAndTests.length) {
            throw new Error("Invalid ciphertexts length");
        }
        mask.status = "removed";
        for (let i = 0; i < this._ciphertexts.length; i++) {
            let [ciphertext, digest] = ciphertextsAndTests[i]!;
            let lockedCiphertext = this._ciphertexts[i]!;
            lockedCiphertext.ciphertext = ciphertext;
            lockedCiphertext.locks.push({ digest, owner: addr });
        }
        let nextMask = this._masks.find(m => m.status === "applied");
        if (nextMask) {
            this._status = { status: "locking", addr: nextMask.owner };
        } else {
            this._status = { status: "ready" };
        }
    }

    public assign(addr: Address, indexes: number[]) {
        if (this._status.status !== "ready" && this._status.status !== "waiting_secrets") {
            throw new Error("Invalid cipher status");
        }
        let duplicated = indexes
            .map(idx => this._ciphertexts[idx])
            .filter((c): c is LockedCiphertext => {
                return !!c && ["assigned", "revealed"].includes(c.owner.type)
            });
        if (duplicated.length > 0) {
            throw new Error("Ciphertext already assigned");
        }
        for (let i of indexes) {
            let lockedCiphertext = this._ciphertexts[i];
            if (lockedCiphertext) {
                lockedCiphertext.owner = { type: "assigned", addr: addr };
            }
            for (let o of this._owners) {
                this._secretShares.push({
                    fromAddr: o,
                    toAddr: addr,
                    index: i,
                    secret: null
                })
            }
        }
        this._status = { status: "waiting_secrets" };
    }

    public reveal(indexes: number[]) {
        if (this._status.status !== "ready" && this._status.status !== "waiting_secrets") {
            throw new Error("Invalid cipher status");
        }
        let duplicated = indexes
            .map(idx => this._ciphertexts[idx])
            .filter((c): c is LockedCiphertext => {
                return !!c && ["revealed"].includes(c.owner.type)
            });
        if (duplicated.length > 0) {
            throw new Error("Ciphertext already assigned");
        }
        for (let i of indexes) {
            let lockedCiphertext = this._ciphertexts[i];
            if (lockedCiphertext) {
                lockedCiphertext.owner = { type: "revealed" };
            }
            for (let o of this._owners) {
                this._secretShares.push({
                    fromAddr: o,
                    toAddr: null,
                    index: i,
                    secret: null
                })
            }
        }
        this._status = { status: "waiting_secrets" };
    }

    public listRequiredSecretsByFrom(fromAddr: Address): SecretIdent[] {
        return this._secretShares
            .filter((s) => s.secret == null && s.fromAddr === fromAddr)
            .map((s) => ({
                fromAddr: s.fromAddr,
                toAddr: s.toAddr,
                randomId: this._id,
                index: s.index
            }));
    }

    public listRevealedSecrets(): Record<number, Ciphertext[]> {
        if (this._status.status !== "ready") {
            throw new Error("Secrets not ready");
        }
        return this._secretShares
            .filter((s) => s.toAddr === null)
            .reduce((acc, s) => {
                if (s.secret) {
                    let secrets = acc[s.index];
                    if (!secrets) {
                        acc[s.index] = [s.secret];
                    } else {
                        secrets.push(s.secret);
                    }
                }
                return acc;
            }, {} as Record<number, Ciphertext[]>);
    }

    public listAssignedCiphertexts(addr: Address): Record<number, Ciphertext> {
        return this._ciphertexts
            .reduce((acc, c, i) => {
                if (c.owner.type === "assigned" && c.owner.addr === addr) {
                    acc[i] = c.ciphertext;
                }
                return acc;
            }, {} as Record<number, Ciphertext>);
    }

    public listRevealedCiphertexts(): Record<number, Ciphertext> {
        return this._ciphertexts
            .reduce((acc, c, i) => {
                if (c.owner.type === "revealed") {
                    acc[i] = c.ciphertext;
                }
                return acc;
            }, {} as Record<number, Ciphertext>);
    }

    public listSharedSecrets(toAddr: Address): Record<number, SecretKey[]> {
        if (this._status.status === "ready") {
            throw new Error("Secrets not ready");
        }
        return this._secretShares
            .reduce((acc, s) => {
                if (s.toAddr === toAddr) {
                    let secrets = acc[s.index];
                    if (!secrets) {
                        acc[s.index] = [s.secret!];
                    } else {
                        secrets.push(s.secret!);
                    }
                }
                return acc;
            }, {} as Record<number, SecretKey[]>);
    }

    public addRevealed(revealed: Record<string, string>) {
        for (let [key, value] of Object.entries(revealed)) {
            let index = Number(key);
            if (index >= this._size) {
                throw new Error("Invalid index");
            }
            this._revealed[index] = value;
        }
    }

    public get revealed() {
        return this._revealed
    }

    public addSecret(fromAddr: Address, toAddr: string | null, index: number, secret: SecretKey) {

        let secretShare = this._secretShares.find(s => {
            s.fromAddr === fromAddr && s.toAddr === toAddr && s.index === index
        });
        if (secretShare) {
            if (secretShare.secret) {
                throw new Error("Duplicated secret");
            } else if (!this._ciphertexts[secretShare.index]) {
                throw new Error("Invalid secret");
            } else {
                secretShare.secret = secret;
            }
        }

        if (this._secretShares.every(s => s.secret)) {
            this._status = { status: "ready" };
        }

    }
}

export function deckOfCards(): ShuffledList {
    return new ShuffledList([
        "ha", "h2", "h3", "h4", "h5", "h6", "h7", "h8", "h9", "ht", "hj", "hq", "hk", "sa", "s2",
        "s3", "s4", "s5", "s6", "s7", "s8", "s9", "st", "sj", "sq", "sk", "da", "d2", "d3", "d4",
        "d5", "d6", "d7", "d8", "d9", "dt", "dj", "dq", "dk", "ca", "c2", "c3", "c4", "c5", "c6",
        "c7", "c8", "c9", "ct", "cj", "cq", "ck",
    ]);
}
