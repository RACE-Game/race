import { Address } from '@solana/web3.js'
import { publicKeyExt } from './utils'
import { deserialize, field, array, struct, option } from '@race-foundation/borsh'

/**
 * A port of Metaplex's Metadata layout.
 *
 * Metaplex library introduces extra dependencies that requires node
 * polyfill, And we only use a small set of its features.
 */
export interface IMetadata {
    key: number
    updateAuthority: Address
    mint: Address
    data: Data
    primarySaleHappened: boolean
    isMutable: boolean
    editionNonce: number | undefined
    tokenStandard: TokenStandard | undefined
    collection: ICollection | undefined
    uses: IUses | undefined
}

export interface ICollection {
    verified: boolean
    key: Address
}

export const USE_METHOD = {
    Burn: 0,
    Multiple: 1,
    Single: 2,
} as const

type UseMethod = (typeof USE_METHOD)[keyof typeof USE_METHOD]

export interface IUses {
    useMethod: UseMethod
    remaining: bigint
    total: bigint
}

export const TOKEN_STANDARD = {
    NonFungible: 0,
    FungibleAsset: 1,
    Fungible: 2,
    NonFungibleEdition: 3,
    ProgrammableNonFungible: 4,
} as const

export type TokenStandard = (typeof TOKEN_STANDARD)[keyof typeof TOKEN_STANDARD]

export interface ICreator {
    address: Address
    verified: boolean
    share: number
}

export interface IData {
    name: string
    symbol: string
    uri: string
    sellerFeeBasisPoints: number
    creators: ICreator[] | undefined
}

export class Creator implements ICreator {
    @field(publicKeyExt)
    address!: Address
    @field('bool')
    verified!: boolean
    @field('u8')
    share!: number
    constructor(fields: ICreator) {
        Object.assign(this, fields)
    }
}

export class Data implements IData {
    @field('string')
    name!: string
    @field('string')
    symbol!: string
    @field('string')
    uri!: string
    @field('u16')
    sellerFeeBasisPoints!: number
    @field(option(array(struct(Creator))))
    creators: ICreator[] | undefined
    constructor(fields: IData) {
        Object.assign(this, fields)
    }
}

export class Collection implements ICollection {
    @field('bool')
    verified!: boolean
    @field(publicKeyExt)
    key!: Address

    constructor(fields: IData) {
        Object.assign(this, fields)
    }
}

export class Uses implements IUses {
    @field('u8')
    useMethod!: UseMethod
    @field('u64')
    remaining!: bigint
    @field('u64')
    total!: bigint

    constructor(fields: IData) {
        Object.assign(this, fields)
    }
}

export class Metadata implements IMetadata {
    @field('u8')
    key!: number
    @field(publicKeyExt)
    updateAuthority!: Address
    @field(publicKeyExt)
    mint!: Address
    @field(struct(Data))
    data!: Data
    @field('bool')
    primarySaleHappened!: boolean
    @field('bool')
    isMutable!: boolean
    @field(option('u8'))
    editionNonce!: number | undefined
    @field(option('u8'))
    tokenStandard!: TokenStandard | undefined
    @field(option(struct(Collection)))
    collection!: Collection | undefined
    @field(option(struct(Uses)))
    uses!: Uses | undefined

    constructor(fields: IMetadata) {
        Object.assign(this, fields)
    }
    static deserialize(data: Uint8Array): Metadata {
        return deserialize(Metadata, new Uint8Array(data.buffer))
    }
}
