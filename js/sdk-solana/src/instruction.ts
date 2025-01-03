import { PublicKey, SYSVAR_RENT_PUBKEY, SystemProgram, TransactionInstruction } from '@solana/web3.js'
import { NATIVE_MINT, TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token'
import { publicKeyExt } from './utils'
import { PROGRAM_ID, METAPLEX_PROGRAM_ID } from './constants'
import { array, enums, field, serialize, struct } from '@race-foundation/borsh'
import { Buffer } from 'buffer'
import { EntryType, Fields, RecipientClaimError, AttachBonusError, Result } from '@race-foundation/sdk-core'
import { AEntryType, RecipientSlotOwner, RecipientSlotOwnerAssigned, RecipientState } from './accounts'

// Instruction types

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
    serialize(): Buffer {
        return Buffer.from(serialize(this))
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
    tokenAddr!: PublicKey

    @field(publicKeyExt)
    stakeAddr!: PublicKey

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
    ownerKey: PublicKey,
    profileKey: PublicKey,
    nick: string,
    pfpKey?: PublicKey
): TransactionInstruction {
    const data = new CreatePlayerProfileData(nick).serialize()

    return new TransactionInstruction({
        keys: [
            {
                pubkey: ownerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: profileKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: pfpKey || PublicKey.default,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data: Buffer.from(data),
    })
}

export type CreateGameOptions = {
    ownerKey: PublicKey
    gameAccountKey: PublicKey
    stakeAccountKey: PublicKey
    recipientAccountKey: PublicKey
    mint: PublicKey
    gameBundleKey: PublicKey
    title: string
    maxPlayers: number
    entryType: EntryType
    data: Uint8Array
}

export type RegisterGameOptions = {
    ownerKey: PublicKey
    gameAccountKey: PublicKey
    registrationAccountKey: PublicKey
}

export function registerGame(opts: RegisterGameOptions): TransactionInstruction {
    const data = Buffer.from(Uint8Array.of(Instruction.RegisterGame))
    return new TransactionInstruction({
        keys: [
            {
                pubkey: opts.ownerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: opts.registrationAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: opts.gameAccountKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export function createGameAccount(opts: CreateGameOptions): TransactionInstruction {
    const params = new CreateGameAccountData({
        title: opts.title,
        entryType: AEntryType.from(opts.entryType),
        maxPlayers: opts.maxPlayers,
        data: opts.data,
    })
    console.debug('Build CreateGameAccount instruction with:', params)
    const data = params.serialize()
    return new TransactionInstruction({
        keys: [
            {
                pubkey: opts.ownerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: opts.gameAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: opts.stakeAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: opts.mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: opts.gameBundleKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: opts.recipientAccountKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export type CloseGameAccountOptions = {
    ownerKey: PublicKey
    gameAccountKey: PublicKey
    regAccountKey: PublicKey
    gameStakeKey: PublicKey
}

export function closeGameAccount(opts: CloseGameAccountOptions): TransactionInstruction {
    const { ownerKey, gameAccountKey, regAccountKey, gameStakeKey } = opts
    const data = new CloseGameAccountData().serialize()
    let [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID)
    return new TransactionInstruction({
        keys: [
            {
                pubkey: ownerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: gameAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: regAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: gameStakeKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: pda,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export type JoinOptions = {
    playerKey: PublicKey
    profileKey: PublicKey
    paymentKey: PublicKey
    gameAccountKey: PublicKey
    mint: PublicKey
    stakeAccountKey: PublicKey
    recipientAccountKey: PublicKey
    amount: bigint
    accessVersion: bigint
    settleVersion: bigint
    position: number
    verifyKey: string
}

export function join(opts: JoinOptions): TransactionInstruction {
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
    } = opts

    let [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID)
    const data = new JoinGameData({ amount, accessVersion, settleVersion, position, verifyKey }).serialize()

    return new TransactionInstruction({
        keys: [
            {
                pubkey: playerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: profileKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: paymentKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: gameAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: stakeAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: recipientAccountKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: pda,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export type DepositOpts = {
    playerKey: PublicKey
    profileKey: PublicKey
    paymentKey: PublicKey
    gameAccountKey: PublicKey
    mint: PublicKey
    stakeAccountKey: PublicKey
    recipientAccountKey: PublicKey
    amount: bigint
    settleVersion: bigint
}

export function deposit(opts: DepositOpts): TransactionInstruction {

    const {
        playerKey,
        profileKey,
        paymentKey,
        gameAccountKey,
        mint,
        stakeAccountKey,
        amount,
        settleVersion
    } = opts;

    let [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID)
    const data = new DepositGameData({ amount, settleVersion }).serialize()

    return new TransactionInstruction({
        keys: [
            {
                pubkey: playerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: profileKey,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: paymentKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: gameAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: stakeAccountKey,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: pda,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export type PublishGameOptions = {
    ownerKey: PublicKey
    mint: PublicKey
    tokenAccountKey: PublicKey
    uri: string
    name: string
    symbol: string
}

export function publishGame(opts: PublishGameOptions): TransactionInstruction {
    const { ownerKey, mint, uri, name, symbol } = opts

    let [metadataPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        METAPLEX_PROGRAM_ID
    )

    let [editonPda] = PublicKey.findProgramAddressSync(
        [
            Buffer.from('metadata', 'utf8'),
            METAPLEX_PROGRAM_ID.toBuffer(),
            mint.toBuffer(),
            Buffer.from('edition', 'utf8'),
        ],
        METAPLEX_PROGRAM_ID
    )
    let ata = getAssociatedTokenAddressSync(mint, ownerKey)

    let data = new PublishGameData({ uri, name, symbol }).serialize()

    return new TransactionInstruction({
        keys: [
            {
                pubkey: ownerKey,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ata,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: metadataPda,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: editonPda,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: METAPLEX_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SYSVAR_RENT_PUBKEY,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: PublicKey.default,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId: PROGRAM_ID,
        data,
    })
}

export type CreateRecipientOpts = {
    payerKey: PublicKey
    capKey: PublicKey
    recipientKey: PublicKey
    slots: SlotInit[]
}

export function createRecipient(opts: CreateRecipientOpts): TransactionInstruction {
    const { payerKey, capKey, recipientKey, slots } = opts

    let keys = [
        {
            pubkey: payerKey,
            isSigner: true,
            isWritable: false,
        },
        {
            pubkey: capKey,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: recipientKey,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
    ]

    slots.forEach(slot => keys.push({ pubkey: slot.stakeAddr, isSigner: false, isWritable: false }))

    const data = new CreateRecipientData({ slots }).serialize()

    return new TransactionInstruction({
        keys,
        programId: PROGRAM_ID,
        data,
    })
}

export type AttachBonusOpts = {
    payerKey: PublicKey
    gameAccountKey: PublicKey
    stakeAccountKey: PublicKey
    tempAccountKeys: PublicKey[]
    identifiers: string[]
}

export function attachBonus(opts: AttachBonusOpts): Result<TransactionInstruction, AttachBonusError> {
    let keys = [
        {
            pubkey: opts.payerKey,
            isSigner: true,
            isWritable: false,
        },
        {
            pubkey: opts.gameAccountKey,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        ...opts.tempAccountKeys.map(k => ({
            pubkey: k,
            isSigner: true,
            isWritable: true,
        }))
    ]

    if (keys.length > 20) {
        return { err: 'too-much-bonuses' }
    }

    const data = new AttachBonusData({
        identifiers: opts.identifiers
    }).serialize()

    return {
        ok: new TransactionInstruction({
            keys,
            programId: PROGRAM_ID,
            data,
        })
    }
}

export type ClaimOpts = {
    payerKey: PublicKey
    recipientKey: PublicKey
    recipientState: RecipientState
}

export function claim(opts: ClaimOpts): Result<TransactionInstruction, RecipientClaimError> {
    const [pda, _] = PublicKey.findProgramAddressSync([opts.recipientKey.toBuffer()], PROGRAM_ID)

    let keys = [
        {
            pubkey: opts.payerKey,
            isSigner: true,
            isWritable: false,
        },
        {
            pubkey: opts.recipientKey,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: pda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ]

    for (const slot of opts.recipientState.slots) {
        for (const slotShare of slot.shares) {
            if (slotShare.owner instanceof RecipientSlotOwnerAssigned && slotShare.owner.addr === opts.payerKey) {
                keys.push({
                    pubkey: slot.stakeAddr,
                    isSigner: false,
                    isWritable: false,
                })

                if (slot.tokenAddr.equals(NATIVE_MINT)) {
                    keys.push({
                        pubkey: opts.payerKey,
                        isSigner: false,
                        isWritable: false,
                    })
                } else {
                    const ata = getAssociatedTokenAddressSync(slotShare.owner.addr, slot.tokenAddr)
                    keys.push({
                        pubkey: ata,
                        isSigner: false,
                        isWritable: false,
                    })
                }
            }
        }
    }

    if (keys.length === 5) {
        return { err: 'no-slots-to-claim' }
    }

    return {
        ok: new TransactionInstruction({
            keys,
            programId: PROGRAM_ID,
            data: Buffer.from(Uint8Array.of(Instruction.RecipientClaim)),
        }),
    }
}
