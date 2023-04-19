import { PublicKey, SYSVAR_RENT_PUBKEY, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import * as borsh from 'borsh';
import { TOKEN_2022_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { ExtendedWriter } from './utils';
import { Metaplex } from '@metaplex-foundation/js';

const PROGRAM_ID = new PublicKey('8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu');
const METAPLEX_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

// Instruction types

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

// Instruction data definitations

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
    super(createGameAccountDataSchema)
    Object.assign(this, params)
  }
}

const createGameAccountDataSchema = new Map([
  [
    CreateGameAccountData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['title', 'string'],
        ['maxPlayers', 'u8'],
        ['minDeposit', 'bigint'],
        ['maxDeposit', 'bigint'],
        ['data', 'bytes']
      ]
    }
  ]
]);

export class JoinGameData extends Serialize {
  instruction = Instruction.JoinGame;
  amount: bigint;
  accessVersion: bigint;
  position: number;

  constructor(amount: bigint, accessVersion: bigint, position: number) {
    super(joinGameDataSchema);
    this.amount = amount;
    this.accessVersion = accessVersion;
    this.position = position;
  }
}

const joinGameDataSchema = new Map([
  [
    JoinGameData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['amount', 'bigint'],
        ['accessVersion', 'bigint'],
        ['position', 'u32'],
      ],
    },
  ],
]);

export class PublishGameData extends Serialize {
  instruction = Instruction.PublishGame;
  uri: string;
  name: string;
  symbol: string;

  constructor(uri: string, name: string, symbol: string) {
    super(publishGameDataSchema);
    this.uri = uri;
    this.name = name;
    this.symbol = symbol;
  }
}

const publishGameDataSchema = new Map([
  [
    PublishGameData,
    {
      kind: 'struct',
      fields: [
        ['uri', 'string'],
        ['name', 'string'],
        ['symbol', 'string'],
      ]
    }
  ]]);

// Instruction helpers

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
        isSigner: false,
        isWritable: true,
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
  ownerKey: PublicKey,
  gameAccountKey: PublicKey,
  stakeAccountKey: PublicKey,
  mint: PublicKey,
  gameBundleKey: PublicKey,
  title: string
  maxPlayers: number
  minDeposit: bigint
  maxDeposit: bigint
};

export function createGameAccount(
  opts: CreateGameOptions
): TransactionInstruction {
  const data = new CreateGameAccountData(opts).serialize();
  return new TransactionInstruction({
    keys: [
      {
        pubkey: opts.ownerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: opts.gameAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: opts.stakeAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: opts.mint,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: TOKEN_2022_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: opts.gameBundleKey,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: PROGRAM_ID,
    data
  });
}

export type CloseGameAccountOptions = {
  ownerKey: PublicKey,
  gameAccountKey: PublicKey,
  regAccountKey: PublicKey,
  gameStakeKey: PublicKey
};

export function closeGameAccount(
  opts: CloseGameAccountOptions
): TransactionInstruction {
  const { ownerKey, gameAccountKey, regAccountKey, gameStakeKey } = opts;
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

export type JoinOptions = {
  playerKey: PublicKey;
  paymentKey: PublicKey;
  gameAccountKey: PublicKey;
  mint: PublicKey;
  stakeAccountKey: PublicKey;
  amount: bigint,
  accessVersion: bigint,
  position: number,
};

export function join(
  opts: JoinOptions
): TransactionInstruction {
  const {
    playerKey,
    paymentKey,
    gameAccountKey,
    mint,
    stakeAccountKey,
    amount,
    accessVersion,
    position
  } = opts;

  let [pda, _] = PublicKey.findProgramAddressSync(
    [gameAccountKey.toBuffer()],
    PROGRAM_ID,
  );
  const data = new JoinGameData(amount, accessVersion, position).serialize();

  return new TransactionInstruction({
    keys: [
      {
        pubkey: playerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: paymentKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: gameAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: mint,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: stakeAccountKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: TOKEN_2022_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: PROGRAM_ID,
    data
  })
}

export type PublishGameOptions = {
  ownerKey: PublicKey,
  mint: PublicKey,
  tokenAccountKey: PublicKey,
  metaplex: Metaplex,
  uri: string,
  name: string,
  symbol: string,
};

export function publishGame(
  opts: PublishGameOptions
): TransactionInstruction {
  const {
    ownerKey, mint, metaplex, uri, name, symbol
  } = opts;

  let metadataPda = metaplex.nfts().pdas().metadata({ mint });
  let editonPda = metaplex.nfts().pdas().masterEdition({ mint });
  let ata = getAssociatedTokenAddressSync(mint, ownerKey);

  let data = new PublishGameData(uri, name, symbol).serialize();

  return new TransactionInstruction({
    keys: [
      {
        pubkey: ownerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: mint,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: ata,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: metadataPda,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: editonPda,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: TOKEN_2022_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: METAPLEX_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: SYSVAR_RENT_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: PublicKey.default,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: PROGRAM_ID,
    data
  });
}
