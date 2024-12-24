import { bcs, SerializedBcs } from '@mysten/bcs'
import { RecipientSlotInit, RecipientSlotShareInit } from '@race-foundation/sdk-core'

export const RecipientSlotOwnerAssignedSchema = bcs.struct('RecipientSlotOwnerAssigned', {
    addr: bcs.string(),
})

export const RecipientSlotOwnerUnassignedSchema = bcs.struct('RecipientSlotOwnerUnassigned', {
    identifier: bcs.string(),
})

export const RecipientSlotOwnerSchema = bcs.enum('RecipientSlotOwner', {
    unassigned: RecipientSlotOwnerUnassignedSchema,
    assigned: RecipientSlotOwnerAssignedSchema,
})

export const RecipientSlotShareSchema = bcs.struct('RecipientSlotShare', {
    owner: RecipientSlotOwnerSchema,
    weights: bcs.u16(),
    claim_amount: bcs.u64(),
})

export const RecipientSlotTypeSchema = bcs.enum('RecipientSlotType', {
    nft: null,
    token: null
})

export type Owner =
  | { assigned: { addr: string }}
  | { unassigned: { identifier: string }}

export interface X {
    owner: Owner
    weights: number
    claim_amount: bigint
}

export function serializeRecipientSlotType(slotType: 'nft' | 'token'): SerializedBcs<any> {
    let slotTypeParsed
    if (slotType === 'nft') {
        slotTypeParsed = { nft: null }
    } else {
        slotTypeParsed = { token: null }
    }
    return RecipientSlotTypeSchema.serialize(slotTypeParsed)
}

export function serializeRecipientSlotShares(shares: RecipientSlotShareInit[]): SerializedBcs<any> {
    const sharesParsed: X[] = shares.map(share => {
        let owner: Owner
        if ('addr' in share.owner) {
            owner = {
                assigned: { addr: share.owner.addr },
            }
        } else {
            owner = {
                unassigned: { identifier: share.owner.identifier },
            }
        }
        return {
            weights: share.weights,
            claim_amount: 0n,
            owner,
        }
    })

    return bcs.vector(RecipientSlotShareSchema).serialize(sharesParsed)
}
