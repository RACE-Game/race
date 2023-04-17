import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import * as borsh from 'borsh';

const PROGRAM_ID = new PublicKey('8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu');

export enum Instruction {
  CreateGameAccount = 0,
  CloseGameAccount = 1,
  CreateRegistration = 2,
  CreatePlayerProfile = 3,
  RegisterServer = 4,
  Settle = 5,
  Vote = 6,
  ServeGame = 7,
  RegisterGame = 8,
  UnregisterGame = 9,
  JoinGame = 10,
  PublishGame = 11,
}

export class CreatePlayerProfileData {
  instruction = Instruction.CreatePlayerProfile;
  nick: string;

  constructor(nick: string) {
    this.nick = nick;
  }
}

export const createPlayerProfileDataScheme = new Map([
  [
    CreatePlayerProfileData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['nick', 'string'],
      ],
    },
  ],
]);

export function createCreatePlayerProfile(
  ownerKey: PublicKey,
  profileKey: PublicKey,
  nick: string,
  pfpKey?: PublicKey
): TransactionInstruction {
  const data = borsh.serialize(createPlayerProfileDataScheme, new CreatePlayerProfileData(nick));

  return new TransactionInstruction({
    keys: [
      {
        pubkey: ownerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: profileKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: pfpKey || PublicKey.default,
        isSigner: false,
        isWritable: false,
      },
    ],
    programId: PROGRAM_ID,
    data: Buffer.from(data),
  });
}
