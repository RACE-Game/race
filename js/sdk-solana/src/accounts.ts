import { field, option, vec } from "@dao-xyz/borsh";
import { PublicKey } from "@solana/web3.js";



export class PlayerState {
  @field({ type: 'bool' })
  is_initialized: boolean;

  @field({ type: 'string' })
  nick: string

  @field({ type: option(PublicKey) })
  pfp?: PublicKey

  @field({ type: vec('u8') })
  padding: Uint8Array

  constructor(data: PlayerState) {
    this.is_initialized = data.is_initialized;
    this.nick = data.nick;
    this.pfp = data.pfp;
    this.padding = data.padding
  }
}
