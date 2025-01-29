import { bcs } from '@mysten/bcs'
import { Address, Parser } from './parser'
import {
    RecipientAccount,
    RecipientSlot,
    RecipientSlotOwner,
    RecipientSlotShare,
    RecipientSlotType,
    RECIPIENT_SLOT_TYPES,
} from '@race-foundation/sdk-core'

// Define the schema for RecipientSlotOwner
const RecipientSlotOwnerSchema = bcs.enum('RecipientSlotOwner', {
    unassigned: bcs.struct('Unassigned', { identifier: bcs.string() }),
    assigned: bcs.struct('Assigned', { addr: Address }),
})

// Define the schema for RecipientSlotShare
const RecipientSlotShareSchema = bcs.struct('RecipientSlotShare', {
    owner: RecipientSlotOwnerSchema,
    weights: bcs.u16(),
    claimAmount: bcs.u64(),
})

const SlotTypeSchema = bcs.enum('RecipientSlotType', {
    Nft: null,
    Token: null
})

// Define the schema for RecipientSlot
const RecipientSlotSchema = bcs.struct('RecipientSlot', {
    id: Address,
    slot_id: bcs.u8(),
    slotType: bcs.u8(), // should map to RecipientSlotType in the transform below
    tokenAddr: bcs.string(),
    shares: bcs.vector(RecipientSlotShareSchema),
    balance: bcs.u64(),
})

// Define the schema for RecipientAccount
const RecipientAccountSchema = bcs.struct('RecipientAccount', {
    addr: Address,
    capAddr: bcs.option(Address),
    slots: bcs.vector(RecipientSlotSchema),
})

// Define the parser for RecipientAccount
export const RecipientAccountParser: Parser<RecipientAccount, typeof RecipientAccountSchema> = {
    schema: RecipientAccountSchema,
    transform: (input: typeof RecipientAccountSchema.$inferType): RecipientAccount => {
        return {
            addr: input.addr,
            capAddr: input.capAddr ? input.capAddr : undefined,
            slots: Array.from(input.slots).map(slot => ({
                id: slot.slot_id,
                slotType: RECIPIENT_SLOT_TYPES[slot.slotType],
                tokenAddr: slot.tokenAddr,
                shares: Array.from(slot.shares).map(share => ({
                    owner: 'unassigned' in share.owner && share.owner.unassigned
                        ? { identifier: share.owner.unassigned.identifier, kind: 'unassigned' }
                        : { addr: share.owner.assigned.addr, kind: 'assigned' },
                    weights: share.weights,
                    claimAmount: BigInt(share.claimAmount)
                })),
                balance: BigInt(slot.balance)
            })),
        }
    },
}
