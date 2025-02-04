import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import {
    Bonus,
    PlayerBalance,
    CheckpointOnChain,
    EntryLock,
    EntryType,
    GameAccount,
    PlayerJoin,
    PlayerDeposit,
    ServerJoin,
    VoteType,
    ENTRY_LOCKS,
    VOTE_TYPES,
    DEPOSIT_STATUS,
} from '@race-foundation/sdk-core'

const BonusSchema = bcs.struct('BonusSchema', {
    id: Address,
    identifier: bcs.string(),
    tokenAddr: bcs.string(),
    amount: bcs.u64(),
})

const BonusParser: Parser<Bonus, typeof BonusSchema> = {
    schema: BonusSchema,
    transform: (input: typeof BonusSchema.$inferType): Bonus => {
        return {
            identifier: input.identifier,
            tokenAddr: input.tokenAddr,
            amount: BigInt(input.amount)
        }
    }
}

const PlayerBalanceSchema = bcs.struct('PlayerBalance', {
    playerId: bcs.u64(),
    balance: bcs.u64(),
})

const PlayerBalanceParser: Parser<PlayerBalance, typeof PlayerBalanceSchema> = {
    schema: PlayerBalanceSchema,
    transform: (input: typeof PlayerBalanceSchema.$inferType): PlayerBalance => {
        return {
            playerId: BigInt(input.playerId),
            balance: BigInt(input.balance),
        }
    }
}

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
const EntryTypeSchema = bcs.enum('EntryTypeSchema', {
    Cash: EntryTypeCashSchema,
    Ticket: EntryTypeTicketSchema,
    Gating: EntryTypeGatingSchema,
    Disabled: null,
})

const EntryTypeParser: Parser<EntryType, typeof EntryTypeSchema> = {
    schema: EntryTypeSchema,
    transform: (input: typeof EntryTypeSchema.$inferType): EntryType => {
        if (input.$kind === 'Cash') {
            return {
                minDeposit: BigInt(input.Cash.min_deposit),
                maxDeposit: BigInt(input.Cash.max_deposit),
                kind: 'cash',
            }
        } else if (input.$kind === 'Ticket') {
            return {
                amount: BigInt(input.Ticket.amount),
                kind: 'ticket',
            }
        } else if (input.$kind == 'Gating') {
            return {
                collection: input.Gating.collection,
                kind: 'gating',
            }
        } else {
            return { kind: 'disabled' }
        }
    }
}

// Define the schema for Game
const GameSchema = bcs.struct('Game', {
    addr: Address,
    version: bcs.string(),
    title: bcs.string(),
    bundleAddr: Address,
    tokenAddr: bcs.string(),
    ownerAddr: Address,
    recipientAddr: Address,
    transactorAddr: bcs.option(Address),
    accessVersion: bcs.u64(),
    settleVersion: bcs.u64(),
    maxPlayers: bcs.u16(),
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
    balance: bcs.u64(),
    dataLen: bcs.u32(),
    data: bcs.vector(bcs.u8()),
    votes: bcs.vector(bcs.struct('Vote', {
        voter: Address,
        votee: Address,
        voteType: bcs.u8(), // This should map to VoteType in the transform function
    })),
    unlockTime: bcs.option(bcs.u64()),
    entryType: EntryTypeSchema,
    checkpointOnChain: bcs.vector(bcs.u8()),
    entryLock: bcs.u8(),  // This should map to EntryLock in the transform function
    bonuses: bcs.vector(BonusSchema),
    balances: bcs.vector(PlayerBalanceSchema),
})

// Transform function to convert from BCS to the TypeScript type
export const GameAccountParser: Parser<GameAccount, typeof GameSchema> = {
    schema: GameSchema,
    transform: (input: typeof GameSchema.$inferType): GameAccount => {

        console.info('input.checkpointOnChain =>', input.checkpointOnChain)
        console.info('input.transactorAddr =>', input.transactorAddr)

        return {
            addr: input.addr,
            title: input.title,
            bundleAddr: input.bundleAddr,
            tokenAddr: input.tokenAddr,
            ownerAddr: input.ownerAddr,
            settleVersion: BigInt(input.settleVersion),
            accessVersion: BigInt(input.accessVersion),
            players: Array.from(input.players).map((player) => ({
                addr: player.addr,
                position: player.position,
                accessVersion: BigInt(player.accessVersion),
                verifyKey: player.verifyKey,
            })),
            deposits: Array.from(input.deposits).map((deposit) => ({
                addr: deposit.addr,
                amount: BigInt(deposit.amount),
                accessVersion: BigInt(deposit.accessVersion),
                settleVersion: BigInt(deposit.settleVersion),
                status: DEPOSIT_STATUS[deposit.status],
            })),
            servers: Array.from(input.servers).map((server) => ({
                addr: server.addr,
                endpoint: server.endpoint,
                accessVersion: BigInt(server.accessVersion),
                verifyKey: server.verifyKey,
            })),
            transactorAddr: input.transactorAddr ?? undefined,
            votes: Array.from(input.votes).map((vote) => ({
                voter: vote.voter,
                votee: vote.votee,
                voteType: VOTE_TYPES[vote.voteType],
            })),
            unlockTime: input.unlockTime ? BigInt(input.unlockTime) : undefined,
            maxPlayers: input.maxPlayers,
            dataLen: input.dataLen,
            data: Uint8Array.from(input.data),
            entryType: EntryTypeParser.transform(input.entryType),
            recipientAddr: input.recipientAddr,
            checkpointOnChain: input.checkpointOnChain.length > 0
                ? CheckpointOnChain.fromRaw(Uint8Array.from(input.checkpointOnChain))
                : undefined,
            entryLock: ENTRY_LOCKS[input.entryLock],
            bonuses: Array.from(input.bonuses).map((bonus) => (
                BonusParser.transform(bonus))
            ),
            balances: Array.from(input.balances).map((balance) => (
                PlayerBalanceParser.transform(balance)
            ))
        }
    },
}
