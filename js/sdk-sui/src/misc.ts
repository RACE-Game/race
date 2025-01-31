import { bcs, BcsType, fromBase64 } from '@mysten/bcs'
import {
    PaginatedObjectsResponse,
    SuiClient,
    SuiObjectChangeCreated,
    SuiObjectChangeMutated,
    SuiObjectData,
    SuiObjectRef,
    SuiObjectResponse,
} from '@mysten/sui/client'
import { SharedObjectRef } from '@mysten/sui/dist/cjs/bcs/types'
import {
    IWallet,
    ResponseHandle,
} from '@race-foundation/sdk-core'
import { LocalSuiWallet } from './local-wallet'
import { ISigner, TxResult } from './signer'
import { SuiWallet } from './sui-wallet'
import {
    Parser,
} from './types'

export function coerceWallet(wallet: IWallet): asserts wallet is ISigner {
    if (!(wallet instanceof LocalSuiWallet) && !(wallet instanceof SuiWallet)) {
        throw new Error('Invalid wallet instance passed')
    }
}

export async function resolveObjectCreatedByType(
    suiClient: SuiClient,
    result: TxResult,
    objectType: string,
): Promise<SuiObjectChangeCreated | undefined> {

    const txBlock = await suiClient.getTransactionBlock({ digest: result.digest, options: { showObjectChanges: true } })

    if (!txBlock.objectChanges) {
        console.warn('Object changes not found in transaction result')
        return undefined
    }

    const objectChange = txBlock.objectChanges.find(c => c.type == 'created' && c.objectType == objectType)
    if (objectChange === undefined || objectChange.type !== 'created') {
        console.warn('Game object is missing')
        return undefined
    }

    return objectChange
}

export async function resolveObjectMutatedByType(
    suiClient: SuiClient,
    result: TxResult,
    objectType: string,
): Promise<SuiObjectChangeMutated | undefined> {

    const txBlock = await suiClient.getTransactionBlock({ digest: result.digest, options: { showObjectChanges: true } })

    if (!txBlock.objectChanges) {
        console.warn('Object changes not found in transaction result')
        return undefined
    }

    const objectChange = txBlock.objectChanges.find(c => c.type == 'mutated' && c.objectType == objectType)
    if (objectChange === undefined || objectChange.type !== 'mutated') {
        console.warn('Game object is missing')
        return undefined
    }

    return objectChange
}

export function parseMultiObjectResponse(resp: PaginatedObjectsResponse): SuiObjectData[] {
    if ('error' in resp) {
        console.error(resp.error)
        return []
    }
    const objData = resp.data.map(x => x.data).filter((x): x is SuiObjectData => !!x)
    return objData
}

export function parseFirstObjectResponse(resp: PaginatedObjectsResponse): SuiObjectData | undefined {
    if ('error' in resp) {
        console.error(resp.error)
        return undefined
    }
    const objData = resp.data.at(0)?.data
    return objData ?? undefined
}

export function parseSingleObjectResponse(resp: SuiObjectResponse): SuiObjectData | undefined {
    if ('error' in resp) {
        console.error(resp.error)
        return undefined
    }
    const objData = resp.data
    return objData ? objData : undefined
}

export async function getOwnedObjectRef(suiClient: SuiClient, owner: string, structType: string): Promise<SuiObjectRef | undefined> {
    const objsRes = await suiClient.getOwnedObjects({ owner, filter: { StructType: structType }, limit: 1})

    if ('error' in objsRes) {
        console.error(objsRes.error, 'Get owned object ref failed')
        return undefined
    }

    if (!objsRes.data) {
        console.error(`Get owned object ref, data not found. owner: ${owner}, structType: ${structType}`)
        return undefined
    }

    if (objsRes.data.length === 0) {
        console.error(`Get owned object ref, object not found. owner: ${owner}, structType: ${structType}`)
        return undefined
    }

    const obj = objsRes.data[0]

    if ('error' in obj) {
        console.error(obj.error, 'Get owned object ref failed')
        return undefined
    }

    if (!obj.data) {
        console.error(`Get owned object ref, object data not found, owner: ${owner}, structType: ${structType}`)
        return undefined
    }
    const digest = obj.data.digest
    const version = obj.data.version

    return {
        objectId: obj.data.objectId,
        digest,
        version,
    }
}

export async function getSharedObjectRef(suiClient: SuiClient, id: string, mutable: boolean): Promise<SharedObjectRef | undefined> {
    const objRes: SuiObjectResponse = await suiClient.getObject({
        id,
        options: { showOwner: true },
    })

    if ('error' in objRes) {
        console.error(objRes.error, 'Get shared object ref failed')
        return undefined
    }

    if (!objRes.data) {
        console.error(`Get shared object ref ${id}, data not found`)
        return undefined
    }

    const owner = objRes.data.owner

    if (!(owner instanceof Object && 'Shared' in owner)) {
        console.warn('object is not a shared object')
        return undefined
    }

    const initialSharedVersion = owner.Shared.initial_shared_version
    return {
        objectId: id,
        initialSharedVersion,
        mutable
    }
}

export function parseObjectData<T, S extends BcsType<S['$inferInput']>>(
    objData: SuiObjectData | undefined,
    parser: Parser<T, S>,
): undefined | T {
    if (objData === undefined) {
        return undefined
    }

    const data = objData.bcs

    if (!data) {
        console.warn('BCS data is empty')
        return undefined
    }

    if (data.dataType === 'package') {
        console.error('Not a move object')
        return undefined
    }

    const raw = parser.schema.parse(fromBase64(data.bcsBytes))

    return parser.transform(raw)
}
