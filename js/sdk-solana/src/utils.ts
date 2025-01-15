import { Address, getAddressDecoder, getAddressEncoder } from '@solana/web3.js'
import { IExtendWriter, IExtendReader, extend } from '@race-foundation/borsh'

class PublicKeyWriter implements IExtendWriter<Address> {
    write(value: Address, buf: Uint8Array, offset: number) {
        const bytes = getAddressEncoder().encode(value)
        buf.set(bytes, offset)
    }
}

class PublicKeyReader implements IExtendReader<Address> {
    read(buf: Uint8Array, offset: number): Address {
        const slice = buf.slice(offset, offset + 32)
        return getAddressDecoder().decode(slice)
    }
}

export const publicKeyExt = extend({
    size: 32,
    writer: new PublicKeyWriter(),
    reader: new PublicKeyReader(),
})
