import { bcs, BcsType, fromHex, toHex } from '@mysten/bcs'

export const Address = bcs.bytes(32).transform({
    input: (val: string) => fromHex(val.replace(/^0x/, '')),
    output: val => `0x${toHex(val)}`,
})

export type Parser<R, S> =
    S extends BcsType<infer T, infer _Input>
        ? {
              schema: S
              transform: (input: T) => R
          }
        : never
