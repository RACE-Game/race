import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import * as borsh from 'borsh';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { ExtendedWriter } from './utils';

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

abstract class Serialize {
  schema: Map<any, any>
  constructor(schema: Map<any, any>) {
    this.schema = schema
  }
  serialize(): Buffer {
    return Buffer.from(borsh.serialize(this.schema, this, ExtendedWriter));
  }
}

export class CreatePlayerProfileData extends Serialize {
  instruction = Instruction.CreatePlayerProfile;
  nick: string;

  constructor(nick: string) {
    super(createPlayerProfileDataScheme)
    this.nick = nick;
  }
}

const createPlayerProfileDataScheme = new Map([
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


export class CloseGameAccountData extends Serialize {
  instruction = Instruction.CloseGameAccount;

  constructor() {
    super(closeGameAccountDataScheme)
  }
}

const closeGameAccountDataScheme = new Map([
  [
    CloseGameAccountData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8']
      ]
    }
  ]]);

export class CreateGameAccountData extends Serialize {
  instruction = Instruction.CreateGameAccount;
  title: string = '';
  maxPlayers: number = 0;
  minDeposit: bigint = 0n;
  maxDeposit: bigint = 0n;
  data: Uint8Array = Uint8Array.from([]);

  constructor(params: Partial<CreateGameAccountData>) {
    super(createGameAccountDataSchema(params.data?.length || 0))
    Object.assign(this, params)
  }
}

function createGameAccountDataSchema(len: number) {
  return new Map([
    [
      CreateGameAccountData,
      {
        kind: 'struct',
        fields: [
          ['instruction', 'u8'],
          ['title', 'string'],
          ['maxPlayers', 'u8'],
          ['minDeposit', 'u64'],
          ['maxDeposit', 'u64'],
          ['data', 'bytes']
        ]
      }
    ]
  ])
};

export function createPlayerProfile(
  ownerKey: PublicKey,
  profileKey: PublicKey,
  nick: string,
  pfpKey?: PublicKey
): TransactionInstruction {
  const data = new CreatePlayerProfileData(nick).serialize();

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
    data,
  });
}

export type CreateGameOptions = {
  title: string
  maxPlayers: number
  minDeposit: bigint
  maxDeposit: bigint
};

export function createGameAccount(
  ownerKey: PublicKey,
  gameAccountKey: PublicKey,
  stakeAccountKey: PublicKey,
  mint: PublicKey,
  gameBundleKey: PublicKey,
  opts: CreateGameOptions
): TransactionInstruction {
  const data = new CreateGameAccountData(opts).serialize();
  return new TransactionInstruction({
    keys: [
      {
        pubkey: ownerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: gameAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: stakeAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: mint,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: TOKEN_2022_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: gameBundleKey,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: PROGRAM_ID,
    data
  });
}

export function closeGameAccount(
  ownerKey: PublicKey,
  gameAccountKey: PublicKey,
  regAccountKey: PublicKey,
  gameStakeKey: PublicKey
): TransactionInstruction {
  const data = new CloseGameAccountData().serialize();
  let [pda, _] = PublicKey.findProgramAddressSync(
    [gameAccountKey.toBuffer()],
    PROGRAM_ID,
  );
  return new TransactionInstruction({
    keys: [
      {
        pubkey: ownerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: gameAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: regAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: gameStakeKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: pda,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: PROGRAM_ID,
    data,
  })
}
