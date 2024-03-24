import { PublicKey, SYSVAR_RENT_PUBKEY, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { publicKeyExt } from './utils';
import { PROGRAM_ID, METAPLEX_PROGRAM_ID, PLAYER_PROFILE_SEED } from './constants';
import { enums, field, serialize } from '@race-foundation/borsh';
import { Buffer } from 'buffer';
import { EntryType } from '@race-foundation/sdk-core';
import { RecipientSlotOwnerAssigned, RecipientState } from './accounts';

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
  CreateRecipient = 12,
  AssignRecipient = 13,
  RecipientClaim = 14,
}

// Instruction data definitations

abstract class Serialize {
  serialize(): Buffer {
    return Buffer.from(serialize(this));
  }
}

export class CreatePlayerProfileData extends Serialize {
  @field('u8')
  instruction = Instruction.CreatePlayerProfile;
  @field('string')
  nick: string;

  constructor(nick: string) {
    super();
    this.nick = nick;
  }
}

export class CloseGameAccountData extends Serialize {
  @field('u8')
  instruction = Instruction.CloseGameAccount;

  constructor() {
    super();
  }
}

export class CreateGameAccountData extends Serialize {
  @field('u8')
  instruction = Instruction.CreateGameAccount;
  @field('string')
  title: string = '';
  @field('u16')
  maxPlayers: number = 0;
  @field(enums(EntryType))
  entryType!: EntryType
  @field('u8-array')
  data: Uint8Array = Uint8Array.from([]);

  constructor(params: Partial<CreateGameAccountData>) {
    super();
    Object.assign(this, params);
  }
}

export class JoinGameData extends Serialize {
  @field('u8')
  instruction = Instruction.JoinGame;
  @field('u64')
  amount: bigint;
  @field('u64')
  accessVersion: bigint;
  @field('u16')
  position: number;
  @field('string')
  verifyKey: string;

  constructor(amount: bigint, accessVersion: bigint, position: number, verifyKey: string) {
    super();
    this.amount = amount;
    this.accessVersion = accessVersion;
    this.position = position;
    this.verifyKey = verifyKey;
  }
}

export class PublishGameData extends Serialize {
  @field('u8')
  instruction = Instruction.PublishGame;
  @field('string')
  uri: string;
  @field('string')
  name: string;
  @field('string')
  symbol: string;

  constructor(uri: string, name: string, symbol: string) {
    super();
    this.uri = uri;
    this.name = name;
    this.symbol = symbol;
  }
}

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
    data: Buffer.from(data),
  });
}

export type CreateGameOptions = {
  ownerKey: PublicKey;
  gameAccountKey: PublicKey;
  stakeAccountKey: PublicKey;
  mint: PublicKey;
  gameBundleKey: PublicKey;
  title: string;
  maxPlayers: number;
  entryType: EntryType;
};

export function createGameAccount(opts: CreateGameOptions): TransactionInstruction {
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
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: opts.gameBundleKey,
        isSigner: false,
        isWritable: false,
      },
    ],
    programId: PROGRAM_ID,
    data,
  });
}

export type CloseGameAccountOptions = {
  ownerKey: PublicKey;
  gameAccountKey: PublicKey;
  regAccountKey: PublicKey;
  gameStakeKey: PublicKey;
};

export function closeGameAccount(opts: CloseGameAccountOptions): TransactionInstruction {
  const { ownerKey, gameAccountKey, regAccountKey, gameStakeKey } = opts;
  const data = new CloseGameAccountData().serialize();
  let [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID);
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
      },
    ],
    programId: PROGRAM_ID,
    data,
  });
}

export type JoinOptions = {
  playerKey: PublicKey;
  paymentKey: PublicKey;
  gameAccountKey: PublicKey;
  mint: PublicKey;
  stakeAccountKey: PublicKey;
  amount: bigint;
  accessVersion: bigint;
  position: number;
  verifyKey: string;
};

export async function join(opts: JoinOptions): Promise<TransactionInstruction> {
  const { playerKey, paymentKey, gameAccountKey, mint, stakeAccountKey, amount, accessVersion, position, verifyKey } =
    opts;

  const profileKey = await PublicKey.createWithSeed(playerKey, PLAYER_PROFILE_SEED, PROGRAM_ID);

  let [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID);
  const data = new JoinGameData(amount, accessVersion, position, verifyKey).serialize();

  return new TransactionInstruction({
    keys: [
      {
        pubkey: playerKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: profileKey,
        isSigner: false,
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
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
    ],
    programId: PROGRAM_ID,
    data,
  });
}

export type PublishGameOptions = {
  ownerKey: PublicKey;
  mint: PublicKey;
  tokenAccountKey: PublicKey;
  uri: string;
  name: string;
  symbol: string;
};

export function publishGame(opts: PublishGameOptions): TransactionInstruction {
  const { ownerKey, mint, uri, name, symbol } = opts;

  let [metadataPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    METAPLEX_PROGRAM_ID
  );

  let [editonPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mint.toBuffer(), Buffer.from('edition', 'utf8')],
    METAPLEX_PROGRAM_ID
  );
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
        pubkey: TOKEN_PROGRAM_ID,
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
      },
    ],
    programId: PROGRAM_ID,
    data,
  });
}


export type ClaimOpts = {
  payerKey: PublicKey,
  recipientKey: PublicKey,
  recipientState: RecipientState,
};

export function claim(opts: ClaimOpts): TransactionInstruction {
  const [pda, _] = PublicKey.findProgramAddressSync([opts.recipientKey.toBuffer()], PROGRAM_ID);

  let keys = [
    {
      pubkey: opts.payerKey,
      isSigner: true,
      isWritable: false,
    },
    {
      pubkey: opts.recipientKey,
      isSigner: false,
      isWritable: true,
    },
    {
      pubkey: pda,
      isSigner: false,
      isWritable: false,
    },
    {
      pubkey: TOKEN_PROGRAM_ID,
      isSigner: false,
      isWritable: false,
    },
    {
      pubkey: SystemProgram.programId,
      isSigner: false,
      isWritable: false,
    },
  ];

  for (const slot of opts.recipientState.slots) {
    for (const slotShare of slot.shares) {
      if (slotShare.owner instanceof RecipientSlotOwnerAssigned && slotShare.owner.addr === opts.payerKey) {
        keys.push({
          pubkey: slot.stakeAddr,
          isSigner: false,
          isWritable: false,
        });
        const ata = getAssociatedTokenAddressSync(slotShare.owner.addr, slot.tokenAddr);
        keys.push({
          pubkey: ata,
          isSigner: false,
          isWritable: false,
        })
      }
    }
  }

  if (keys.length === 5) {
    throw new Error('No slot to claim');
  }

  return new TransactionInstruction({
    keys,
    programId: PROGRAM_ID,
    data: Buffer.from(Uint8Array.of(Instruction.RecipientClaim)),
  });
}
