import * as SPL from '@solana-program/token'
import * as SYSTEM from '@solana-program/system'
import { publicKeyExt } from './utils'
import { PROGRAM_ID, METAPLEX_PROGRAM_ID, SYSVAR_RENT, NATIVE_MINT } from './constants'
import { array, enums, field, serialize, struct } from '@race-foundation/borsh'
import { EntryType, Fields, RecipientClaimError, AttachBonusError, Result } from '@race-foundation/sdk-core'
import { AEntryType, GameState, RecipientSlotOwner, RecipientSlotOwnerAssigned, RecipientState } from './accounts'
import {
    AccountRole,
    address,
    Address,
    IInstruction,
    getProgramDerivedAddress,
    getBase58Encoder,
} from '@solana/web3.js'

type IxParams<T> = Omit<Fields<T>, 'instruction'>

export enum Instruction {
    CreateGameAccount = 0,
    CloseGameAccount = 1,
    CreateRegistration = 2,
    CreatePlayerProfile = 3,
    RegisterServer = 4,
    Settle = 5,
    Vote = 6,
    ServeGame = 7,
    RegisterGame = 8,
    UnregisterGame = 9,
    JoinGame = 10,
    PublishGame = 11,
    CreateRecipient = 12,
    AssignRecipient = 13,
    RecipientClaim = 14,
    Deposit = 15,
    AttachBonus = 16,
    RejectDeposits = 17,
}

// Instruction data definitations

abstract class Serialize {
    serialize(): Uint8Array {
        return serialize(this)
    }
}

export class CreatePlayerProfileData extends Serialize {
    @field('u8')
    instruction = Instruction.CreatePlayerProfile

    @field('string')
    nick: string

    constructor(nick: string) {
        super()
        this.nick = nick
    }
}

export class CloseGameAccountData extends Serialize {
    @field('u8')
    instruction = Instruction.CloseGameAccount

    constructor() {
        super()
    }
}

export class CreateGameAccountData extends Serialize {
    @field('u8')
    instruction = Instruction.CreateGameAccount
    @field('string')
    title: string = ''
    @field('u16')
    maxPlayers: number = 0
    @field(enums(AEntryType))
    entryType!: AEntryType
    @field('u8-array')
    data: Uint8Array = Uint8Array.from([])

    constructor(params: IxParams<CreateGameAccountData>) {
        super()
        Object.assign(this, params)
    }
}

export class JoinGameData extends Serialize {
    @field('u8')
    instruction = Instruction.JoinGame
    @field('u64')
    amount: bigint
    @field('u64')
    accessVersion: bigint
    @field('u64')
    settleVersion: bigint
    @field('u16')
    position: number
    @field('string')
    verifyKey: string

    constructor(params: IxParams<JoinGameData>) {
        super()
        this.amount = params.amount
        this.accessVersion = params.accessVersion
        this.settleVersion = params.settleVersion
        this.position = params.position
        this.verifyKey = params.verifyKey
    }
}

export class DepositGameData extends Serialize {
    @field('u8')
    instruction = Instruction.Deposit
    @field('u64')
    amount: bigint
    @field('u64')
    settleVersion: bigint

    constructor(params: IxParams<DepositGameData>) {
        super()
        this.amount = params.amount
        this.settleVersion = params.settleVersion
    }
}

export class PublishGameData extends Serialize {
    @field('u8')
    instruction = Instruction.PublishGame
    @field('string')
    uri: string
    @field('string')
    name: string
    @field('string')
    symbol: string

    constructor(params: IxParams<PublishGameData>) {
        super()
        this.uri = params.uri
        this.name = params.name
        this.symbol = params.symbol
    }
}

export class SlotShareInit {
    @field(enums(RecipientSlotOwner))
    owner!: RecipientSlotOwner

    @field('u16')
    weights!: number

    constructor(fields: any) {
        Object.assign(this, fields)
    }
}

export class SlotInit {
    @field('u8')
    id!: number

    @field('u8')
    slotType!: 0 | 1

    @field(publicKeyExt)
    tokenAddr!: Address

    @field(publicKeyExt)
    stakeAddr!: Address

    @field(array(struct(SlotShareInit)))
    initShares!: SlotShareInit[]

    constructor(fields: Fields<SlotInit>) {
        Object.assign(this, fields)
    }
}

export class CreateRecipientData extends Serialize {
    @field('u8')
    instruction = Instruction.CreateRecipient

    @field(array(struct(SlotInit)))
    slots: SlotInit[]

    constructor(params: IxParams<CreateRecipientData>) {
        super()
        this.slots = params.slots
    }
}

export class AttachBonusData extends Serialize {
    @field('u8')
    instruction = Instruction.AttachBonus

    @field(array('string'))
    identifiers: string[]

    constructor(params: IxParams<AttachBonusData>) {
        super()
        this.identifiers = params.identifiers
    }
}

// Instruction helpers

export function createPlayerProfile(
    ownerKey: Address,
    profileKey: Address,
    nick: string,
    pfpKey?: Address
): IInstruction {
    const data = new CreatePlayerProfileData(nick).serialize()

    return {
        accounts: [
            {
                address: ownerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: profileKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: pfpKey || address(''),
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type CreateGameOptions = {
    ownerKey: Address
    gameAccountKey: Address
    stakeAccountKey: Address
    recipientAccountKey: Address
    mint: Address
    gameBundleKey: Address
    title: string
    maxPlayers: number
    entryType: EntryType
    data: Uint8Array
}

export type RegisterGameOptions = {
    ownerKey: Address
    gameAccountKey: Address
    registrationAccountKey: Address
}

export function registerGame(opts: RegisterGameOptions): IInstruction {
    const data = Uint8Array.of(Instruction.RegisterGame)
    return {
        accounts: [
            {
                address: opts.ownerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: opts.registrationAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: opts.gameAccountKey,
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export function createGameAccount(opts: CreateGameOptions): IInstruction {
    const params = new CreateGameAccountData({
        title: opts.title,
        entryType: AEntryType.from(opts.entryType),
        maxPlayers: opts.maxPlayers,
        data: opts.data,
    })
    console.debug('Build CreateGameAccount instruction with:', params)
    const data = params.serialize()
    return {
        accounts: [
            {
                address: opts.ownerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: opts.gameAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: opts.stakeAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: opts.mint,
                role: AccountRole.READONLY,
            },
            {
                address: SPL.TOKEN_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
            {
                address: opts.gameBundleKey,
                role: AccountRole.READONLY,
            },
            {
                address: opts.recipientAccountKey,
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type CloseGameAccountOptions = {
    pda: Address
    ownerKey: Address
    gameAccountKey: Address
    regAccountKey: Address
    gameStakeKey: Address
}

export function closeGameAccount(opts: CloseGameAccountOptions): IInstruction {
    const { ownerKey, gameAccountKey, regAccountKey, gameStakeKey, pda } = opts
    const data = new CloseGameAccountData().serialize()
    // const [pda, _]  = await getProgramDerivedAddress({ programAddress: PROGRAM_ID, seeds: [gameAccountKey] })
    return {
        accounts: [
            {
                address: ownerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: gameAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: regAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: gameStakeKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: pda,
                role: AccountRole.READONLY,
            },
            {
                address: PROGRAM_ID,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type JoinOptions = {
    playerKey: Address
    profileKey: Address
    paymentKey: Address
    gameAccountKey: Address
    mint: Address
    stakeAccountKey: Address
    recipientAccountKey: Address
    amount: bigint
    accessVersion: bigint
    settleVersion: bigint
    position: number
    verifyKey: string
    pda: Address
}

export function join(opts: JoinOptions): IInstruction {
    const {
        playerKey,
        profileKey,
        paymentKey,
        gameAccountKey,
        mint,
        stakeAccountKey,
        recipientAccountKey,
        amount,
        accessVersion,
        settleVersion,
        position,
        verifyKey,
        pda,
    } = opts

    const data = new JoinGameData({ amount, accessVersion, settleVersion, position, verifyKey }).serialize()

    return {
        accounts: [
            {
                address: playerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: profileKey,
                role: AccountRole.READONLY,
            },
            {
                address: paymentKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: gameAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: mint,
                role: AccountRole.READONLY,
            },
            {
                address: stakeAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: recipientAccountKey,
                role: AccountRole.READONLY,
            },
            {
                address: pda,
                role: AccountRole.WRITABLE,
            },
            {
                address: SPL.TOKEN_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type DepositOpts = {
    playerKey: Address
    profileKey: Address
    paymentKey: Address
    gameAccountKey: Address
    mint: Address
    stakeAccountKey: Address
    recipientAccountKey: Address
    amount: bigint
    settleVersion: bigint
    pda: Address
}

export function deposit(opts: DepositOpts): IInstruction {
    const { playerKey, profileKey, paymentKey, gameAccountKey, mint, stakeAccountKey, amount, settleVersion, pda } =
        opts

    const data = new DepositGameData({ amount, settleVersion }).serialize()

    return {
        accounts: [
            {
                address: playerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: profileKey,
                role: AccountRole.READONLY,
            },
            {
                address: paymentKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: gameAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: mint,
                role: AccountRole.READONLY,
            },
            {
                address: stakeAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: pda,
                role: AccountRole.WRITABLE,
            },
            {
                address: SPL.TOKEN_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type PublishGameOptions = {
    ownerKey: Address
    mint: Address
    tokenAccountKey: Address
    uri: string
    name: string
    symbol: string
    metadataPda: Address
    ata: Address
    editionPda: Address
}

export function publishGame(opts: PublishGameOptions): IInstruction {
    const { ownerKey, mint, uri, name, symbol, metadataPda, editionPda, ata } = opts

    // let [metadataPda] = PublicKey.findProgramAddressSync(
    //     [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    //     METAPLEX_PROGRAM_ID
    // )

    // let [editonPda] = PublicKey.findProgramAddressSync(
    //     [
    //         Buffer.from('metadata', 'utf8'),
    //         METAPLEX_PROGRAM_ID.toBuffer(),
    //         mint.toBuffer(),
    //         Buffer.from('edition', 'utf8'),
    //     ],
    //     METAPLEX_PROGRAM_ID
    // )

    let data = new PublishGameData({ uri, name, symbol }).serialize()

    return {
        accounts: [
            {
                address: ownerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: mint,
                role: AccountRole.READONLY,
            },
            {
                address: ata,
                role: AccountRole.WRITABLE,
            },
            {
                address: metadataPda,
                role: AccountRole.READONLY,
            },
            {
                address: editionPda,
                role: AccountRole.READONLY,
            },
            {
                address: SPL.TOKEN_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
            {
                address: METAPLEX_PROGRAM_ID,
                role: AccountRole.READONLY,
            },
            {
                address: SYSVAR_RENT,
                role: AccountRole.READONLY,
            },
            {
                address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
                role: AccountRole.READONLY,
            },
        ],
        programAddress: PROGRAM_ID,
        data,
    }
}

export type CreateRecipientOpts = {
    payerKey: Address
    capKey: Address
    recipientKey: Address
    slots: SlotInit[]
}

export function createRecipient(opts: CreateRecipientOpts): IInstruction {
    const { payerKey, capKey, recipientKey, slots } = opts

    let accounts = [
        {
            address: payerKey,
            role: AccountRole.READONLY_SIGNER,
        },
        {
            address: capKey,
            role: AccountRole.READONLY,
        },
        {
            address: recipientKey,
            role: AccountRole.READONLY,
        },
        {
            address: SPL.TOKEN_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
        {
            address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        }
    ]

    slots.forEach(slot => accounts.push({ address: slot.stakeAddr, role: AccountRole.READONLY }))

    const data = new CreateRecipientData({ slots }).serialize()

    return {
        accounts,
        programAddress: PROGRAM_ID,
        data,
    }
}

export type AttachBonusOpts = {
    payerKey: Address
    gameAccountKey: Address
    stakeAccountKey: Address
    tempAccountKeys: Address[]
    identifiers: string[]
}

export function attachBonus(opts: AttachBonusOpts): Result<IInstruction, AttachBonusError> {
    let accounts = [
        {
            address: opts.payerKey,
            role: AccountRole.READONLY_SIGNER,
        },
        {
            address: opts.gameAccountKey,
            role: AccountRole.WRITABLE,
        },
        {
            address: SPL.TOKEN_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
        {
            address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
        ...opts.tempAccountKeys.map(k => ({
            address: k,
            role: AccountRole.WRITABLE_SIGNER,
        })),
    ]

    if (accounts.length > 20) {
        return { err: 'too-much-bonuses' }
    }

    const data = new AttachBonusData({
        identifiers: opts.identifiers,
    }).serialize()

    return {
        ok: {
            accounts,
            programAddress: PROGRAM_ID,
            data,
        },
    }
}

export type ClaimOpts = {
    payerKey: Address
    recipientKey: Address
    recipientState: RecipientState
}

export async function claim(opts: ClaimOpts): Promise<Result<IInstruction, RecipientClaimError>> {
    const {

    } = opts

    let accounts = [
        {
            address: opts.payerKey,
            role: AccountRole.READONLY_SIGNER,
        },
        {
            address: opts.recipientKey,
            role: AccountRole.WRITABLE,
        },
        {
            address: SPL.TOKEN_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
        {
            address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
    ]

    for (const slot of opts.recipientState.slots) {
        const [pda, _] = await getProgramDerivedAddress({ programAddress: PROGRAM_ID, seeds: [getBase58Encoder().encode(opts.recipientKey), Uint8Array.of(slot.id)] })

        for (const slotShare of slot.shares) {
            if (slotShare.owner instanceof RecipientSlotOwnerAssigned && slotShare.owner.addr === opts.payerKey) {
                accounts.push({
                    address: pda,
                    role: AccountRole.READONLY,
                })

                accounts.push({
                    address: slot.stakeAddr,
                    role: AccountRole.WRITABLE,
                })

                if (slot.tokenAddr == NATIVE_MINT) {
                    accounts.push({
                        address: opts.payerKey,
                        role: AccountRole.WRITABLE,
                    })
                } else {

                    const [ata] = await SPL.findAssociatedTokenPda({
                        mint: address(slot.tokenAddr),
                        owner: address(slotShare.owner.addr),
                        tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS
                    })
                    accounts.push({
                        address: ata,
                        role: AccountRole.WRITABLE,
                    })
                }
            }
        }
    }

    if (accounts.length === 5) {
        return { err: 'no-slots-to-claim' }
    }

    return {
        ok: {
            accounts,
            programAddress: PROGRAM_ID,
            data: Uint8Array.of(Instruction.RecipientClaim),
        },
    }
}

export type UnregisterGameOpts = {
    payerKey: Address
    regAccountKey: Address
    gameAccountKey: Address
}

export function unregisterGame(opts: UnregisterGameOpts): IInstruction {
    const { payerKey, regAccountKey, gameAccountKey } = opts

    return {
        accounts: [
            {
                address: payerKey,
                role: AccountRole.READONLY_SIGNER,
            },
            {
                address: regAccountKey,
                role: AccountRole.WRITABLE,
            },
            {
                address: gameAccountKey,
                role: AccountRole.READONLY,
            },
        ],
        data: Uint8Array.of(Instruction.UnregisterGame),
        programAddress: PROGRAM_ID,
    }
}

export type CloseGameAccountOpts = {
    payerKey: Address
    gameAccountKey: Address
    stakeKey: Address
    pda: Address
    receiver: Address
    gameState: GameState
}

export async function closeGame(opts: CloseGameAccountOpts): Promise<IInstruction> {
    const { payerKey, gameAccountKey, stakeKey, pda, receiver, gameState } = opts

    let accounts = [
        {
            address: payerKey,
            role: AccountRole.READONLY_SIGNER,
        },
        {
            address: gameAccountKey,
            role: AccountRole.WRITABLE,
        },
        {
            address: stakeKey,
            role: AccountRole.WRITABLE,
        },
        {
            address: pda,
            role: AccountRole.READONLY,
        },
        {
            address: receiver,
            role: AccountRole.WRITABLE,
        },
        {
            address: SPL.TOKEN_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
        {
            address: SYSTEM.SYSTEM_PROGRAM_ADDRESS,
            role: AccountRole.READONLY,
        },
    ]

    for (const bonus of gameState.bonuses) {
        const [ata] = await SPL.findAssociatedTokenPda({
            mint: address(bonus.tokenAddr),
            owner: address(payerKey),
            tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS
        })

        accounts.push({
            address: bonus.stakeAddr,
            role: AccountRole.WRITABLE,
        }, {
            address: ata,
            role: AccountRole.WRITABLE
        })
    }

    return {
        accounts,
        data: Uint8Array.of(Instruction.CloseGameAccount),
        programAddress: PROGRAM_ID,
    }
}
