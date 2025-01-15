import { Address, createJsonRpcApi, createRpc, Rpc, RpcRequest, RpcResponse, RpcResponseData, RpcTransport } from '@solana/web3.js'

type Asset = Readonly<{
    interface: any
    id: Address
    token_info: {
        decimals: number
    }
    grouping: Readonly<{
        group_key: string
        group_value: string
    }[]>
    content: Readonly<{
        files?: readonly {
            mime?: string
            uri?: string
            [key: string]: unknown
        }[]
        json_uri: string
        links?: {
            [key: string]: unknown
        }
        metadata: {
            name: string
            symbol: string
        }
    }>
}>

// Define the method's response payload.
type GetAssetResponse = Asset

type GetAssetsByOwnerResponse = Readonly<{
    cursor: Address
    length: number
    items: Asset[]
}>

type SortCriteria = {
    sortBy: 'created' | 'updated' | 'recentAction' | 'none'
    sortDirection: 'asc' | 'desc'
}

// Set up a type spec for the request method.
export type MetaplexDASApi = {
    getAsset(id: Address): RpcResponseData<GetAssetResponse>;

    getAssetsByOwner(
        input: {
            ownerAddress: Address,
            sortBy?: SortCriteria,
            limit?: number,
            page?: number,
            before?: Address,
            after?: Address,
        }
    ): RpcResponseData<GetAssetsByOwnerResponse>
};

export function createDasRpc(transport: RpcTransport): Rpc<MetaplexDASApi> {
    const jsonRpcConfig = {
        requestTransformer: (request: any) => {
            if ('object' === typeof request.params[0]) {
                return {
                    methodName: request.methodName,
                    params: request.params[0],
                }
            } else {
                return request
            }
        }
    }
    const api = createJsonRpcApi<MetaplexDASApi>(jsonRpcConfig)
    return createRpc({ api, transport })
}
