import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import {
    EntryLock,
    VoteType,
    PlayerJoin,
    PlayerDeposit,
    ServerJoin,
    Bonus,
    GameAccount,
    EntryType,
    ENTRY_LOCKS,
    VOTE_TYPES,
    DEPOSIT_STATUS,
} from '@race-foundation/sdk-core'

const EntryTypeCashSchema = bcs.struct('EntryTypeCash', {
    min_deposit: bcs.u64(), max_deposit: bcs.u64()
})

const EntryTypeTicketSchema = bcs.struct('EntryTypeTicket', {
    amount: bcs.u64()
})

const EntryTypeGatingSchema = bcs.struct('EntryTypeCash', {
    collection: bcs.string()
})

// Define the schema for EntryType
const EntryTypeSchema = bcs.enum('EntryType', {
    Cash: EntryTypeCashSchema,
    Ticket: EntryTypeTicketSchema,
    Gating: EntryTypeGatingSchema,
    Disabled: null,
})

// Define the schema for Game
const GameSchema = bcs.struct('Game', {
    addr: Address,
    title: bcs.string(),
    bundleAddr: Address,
    tokenAddr: bcs.string(),
    ownerAddr: Address,
    settleVersion: bcs.u64(),
    accessVersion: bcs.u64(),
    players: bcs.vector(bcs.struct('PlayerJoin', {
        addr: Address,
        position: bcs.u16(),
        accessVersion: bcs.u64(),
        verifyKey: bcs.string(),
    })),
    deposits: bcs.vector(bcs.struct('PlayerDeposit', {
        addr: Address,
        amount: bcs.u64(),
        accessVersion: bcs.u64(),
        settleVersion: bcs.u64(),
        status: bcs.u8(), // This should map to DepositStatus in the transform function
    })),
    servers: bcs.vector(bcs.struct('ServerJoin', {
        addr: Address,
        endpoint: bcs.string(),
        accessVersion: bcs.u64(),
        verifyKey: bcs.string(),
    })),
    transactorAddr: bcs.option(Address),
    votes: bcs.vector(bcs.struct('Vote', {
        voter: Address,
        votee: Address,
        voteType: bcs.u8(), // This should map to VoteType in the transform function
    })),
    unlockTime: bcs.option(bcs.u64()),
    maxPlayers: bcs.u16(),
    dataLen: bcs.u32(),
    data: bcs.vector(bcs.u8()),
    entryType: EntryTypeSchema,
    recipientAddr: Address,
    checkpointOnChain: bcs.option(Address), // Adjust depending on actual CheckpointOnChain representation
    entryLock: bcs.u8(),                      // This should map to EntryLock in the transform function
    bonuses: bcs.vector(bcs.struct('Bonus', {
        identifier: bcs.string(),
        tokenAddr: bcs.string(),
        amount: bcs.u64(),
    })),
})

// Transform function to convert from BCS to the TypeScript type
export const GameAccountParser: Parser<GameAccount, typeof GameSchema> = {
    schema: GameSchema,
    transform: (input: typeof GameSchema.$inferType): GameAccount => {
        return {
            addr: input.addr,
            title: input.title,
            bundleAddr: input.bundleAddr,
            tokenAddr: input.tokenAddr,
            ownerAddr: input.ownerAddr,
            settleVersion: BigInt(input.settleVersion),
            accessVersion: BigInt(input.accessVersion),
            players: input.players.map((player) => ({
                addr: player.addr,
                position: player.position,
                accessVersion: BigInt(player.accessVersion),
                verifyKey: player.verifyKey,
            })),
            deposits: input.deposits.map((deposit) => ({
                addr: deposit.addr,
                amount: BigInt(deposit.amount),
                accessVersion: BigInt(deposit.accessVersion),
                settleVersion: BigInt(deposit.settleVersion),
                status: DEPOSIT_STATUS[deposit.status],
            })),
            servers: input.servers.map((server) => ({
                addr: server.addr,
                endpoint: server.endpoint,
                accessVersion: BigInt(server.accessVersion),
                verifyKey: server.verifyKey,
            })),
            transactorAddr: input.transactorAddr ?? undefined,
            votes: input.votes.map((vote) => ({
                voter: vote.voter,
                votee: vote.votee,
                voteType: VOTE_TYPES[vote.voteType],
            })),
            unlockTime: input.unlockTime ? BigInt(input.unlockTime) : undefined,
            maxPlayers: input.maxPlayers,
            dataLen: input.dataLen,
            data: Uint8Array.from(input.data),
            entryType: input.entryType, // Process based on the actual EntryTypeKind
            recipientAddr: input.recipientAddr,
            checkpointOnChain: input.checkpointOnChain ?? undefined,
            entryLock: ENTRY_LOCKS[input.entryLock],
            bonuses: input.bonuses.map((bonus) => ({
                identifier: bonus.identifier,
                tokenAddr: bonus.tokenAddr,
                amount: BigInt(bonus.amount),
            })),
        }
    },
}
