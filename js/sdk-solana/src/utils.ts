import { PublicKey } from '@solana/web3.js'
import { IExtendWriter, IExtendReader, extend } from '@race-foundation/borsh'

class PublicKeyWriter implements IExtendWriter<PublicKey> {
    write(value: PublicKey, buf: Uint8Array, offset: number) {
        buf.set(value.toBytes(), offset)
    }
}

class PublicKeyReader implements IExtendReader<PublicKey> {
    read(buf: Uint8Array, offset: number): PublicKey {
        const slice = buf.slice(offset, offset + 32)
        return new PublicKey(slice)
    }
}

export const publicKeyExt = extend({
    size: 32,
    writer: new PublicKeyWriter(),
    reader: new PublicKeyReader(),
})
