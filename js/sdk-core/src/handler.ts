import { array, deserialize, enums, field, serialize, struct } from '@race-foundation/borsh';
import { EntryType, GameAccount, GameBundle } from './accounts';
import { AnswerDecision, GameEvent, GameStart, Leave, Mask, Lock, SecretsReady, ShareSecrets, Sync } from './events';
import { GameContext } from './game-context';
import { IEncryptor } from './encryptor';
import { Effect, GamePlayer } from './effect';
import { Client } from './client';
import { DecryptionCache } from './decryption-cache';

/**
 * A subset of GameAccount, used in handler initialization.
 */
export interface IInitAccount {
  addr: string;
  players: GamePlayer[];
  data: Uint8Array;
  accessVersion: bigint;
  settleVersion: bigint;
  maxPlayers: number;
  checkpoint: Uint8Array;
  entryType: EntryType;
}

export class InitAccount {
  @field('string')
  readonly addr: string;
  @field(array(struct(GamePlayer)))
  readonly players: GamePlayer[];
  @field('u8-array')
  readonly data: Uint8Array;
  @field('u64')
  readonly accessVersion: bigint;
  @field('u64')
  readonly settleVersion: bigint;
  @field('u16')
  readonly maxPlayers: number;
  @field('u8-array')
  readonly checkpoint: Uint8Array;
  @field(enums(EntryType))
  readonly entryType: EntryType;

  constructor(fields: IInitAccount) {
    this.addr = fields.addr;
    this.accessVersion = fields.accessVersion;
    this.settleVersion = fields.settleVersion;
    this.data = fields.data;
    this.players = fields.players;
    this.maxPlayers = fields.maxPlayers;
    this.checkpoint = fields.checkpoint;
    this.entryType = fields.entryType;
  }
  static createFromGameAccount(
    gameAccount: GameAccount,
    transactorAccessVersion: bigint,
    transactorSettleVersion: bigint
  ): InitAccount {
    let { addr, players, data, checkpointAccessVersion } = gameAccount;
    const game_players = players.filter(p => p.accessVersion <= checkpointAccessVersion)
      .map(p => new GamePlayer({ id: p.accessVersion, balance: p.balance, position: p.position }));
    return new InitAccount({
      addr,
      data,
      players: game_players,
      accessVersion: transactorAccessVersion,
      settleVersion: transactorSettleVersion,
      maxPlayers: gameAccount.maxPlayers,
      checkpoint: gameAccount.checkpoint,
      entryType: gameAccount.entryType,
    });
  }
  serialize(): Uint8Array {
    return serialize(InitAccount);
  }
  static deserialize(data: Uint8Array) {
    return deserialize(InitAccount, data);
  }
}

export interface IHandler {
  handleEvent(context: GameContext, event: GameEvent): Promise<void>;

  initState(context: GameContext, initAccount: InitAccount): Promise<void>;
}

export class Handler implements IHandler {
  #encryptor: IEncryptor;
  #instance: WebAssembly.Instance;
  #client: Client;
  #decryptionCache: DecryptionCache;

  constructor(instance: WebAssembly.Instance, encryptor: IEncryptor, client: Client, decryptionCache: DecryptionCache) {
    this.#encryptor = encryptor;
    this.#instance = instance;
    this.#client = client;
    this.#decryptionCache = decryptionCache;
  }

  static async initialize(gameBundle: GameBundle, encryptor: IEncryptor, client: Client, decryptionCache: DecryptionCache): Promise<Handler> {
    const importObject = {
      imports: {
        memory: new WebAssembly.Memory({
          shared: true,
          maximum: 100,
          initial: 100,
        }),
      },
    };
    let initiatedSource;
    if (gameBundle.data.length === 0) {
      console.debug('Initiate handler by streaming:', gameBundle.uri);
      initiatedSource = await WebAssembly.instantiateStreaming(fetch(gameBundle.uri), importObject);
    } else {
      initiatedSource = await WebAssembly.instantiate(gameBundle.data, importObject);
    }
    return new Handler(initiatedSource.instance, encryptor, client, decryptionCache);
  }

  async handleEvent(context: GameContext, event: GameEvent) {
    await this.generalPreHandleEvent(context, event, this.#encryptor);
    await this.customHandleEvent(context, event);
    await this.generalPostHandleEvent(context, event);
    context.applyAndTakeSettles();
  }

  async initState(context: GameContext, initAccount: InitAccount) {
    await this.generalPreInitState(context, initAccount);
    await this.customInitState(context, initAccount);
    await this.generalPostInitState(context, initAccount);
  }

  async generalPreInitState(_context: GameContext, _initAccount: InitAccount) {}

  async generalPostInitState(_context: GameContext, _initAccount: InitAccount) {}

  async generalPreHandleEvent(context: GameContext, event: GameEvent, encryptor: IEncryptor) {
    if (event instanceof ShareSecrets) {
      const { sender, shares } = event;
      const addr = context.idToAddr(sender);
      context.addSharedSecrets(addr, shares);
      let randomIds: number[] = [];
      for (let randomState of context.randomStates) {
        if (randomState.status.kind === 'shared') {
          randomIds.push(randomState.id);
          randomState.status = { kind: 'ready' };
        }
      }
      if (randomIds.length > 0) {
        context.dispatchEventInstantly(new SecretsReady({ randomIds }));
      }
    } else if (event instanceof AnswerDecision) {
      const { decisionId, ciphertext, sender, digest } = event;
      const addr = context.idToAddr(sender);
      context.answerDecision(decisionId, addr, ciphertext, digest);
    } else if (event instanceof Mask) {
      const { sender, randomId, ciphertexts } = event;
      const addr = context.idToAddr(sender);
      context.randomizeAndMask(addr, randomId, ciphertexts);
    } else if (event instanceof Lock) {
      const { sender, randomId, ciphertextsAndDigests } = event;
      const addr = context.idToAddr(sender);
      context.lock(addr, randomId, ciphertextsAndDigests);
    } else if (event instanceof Sync) {
      // No op here
    } else if (event instanceof Leave) {
      if (!context.allowExit) {
        throw new Error('Leave is not allowed')
      }
    } else if (event instanceof GameStart) {
      const { accessVersion } = event;
      context.status = 'running';
      context.setNodeReady(accessVersion);
    } else if (event instanceof SecretsReady) {
      for (let randomId of event.randomIds) {
        let decryption = await this.#client.decrypt(context, randomId);
        this.#decryptionCache.add(randomId, decryption);
      }

      for (const st of context.randomStates) {
        const options = st.options;
        const revealed = await encryptor.decryptWithSecrets(
          st.listRevealedCiphertexts(),
          st.listRevealedSecrets(),
          options
        );
        context.addRevealedRandom(st.id, revealed);
      }
    }
  }

  async generalPostHandleEvent(context: GameContext, _event: GameEvent) {
    if (context.checkpoint) {
      context.randomStates = [];
      context.decisionStates = [];
    }
  }

  async customInitState(context: GameContext, initAccount: InitAccount) {
    const exports = this.#instance.exports;
    const mem = exports.memory as WebAssembly.Memory;
    mem.grow(4);
    let buf = new Uint8Array(mem.buffer);

    const effect = Effect.fromContext(context);
    const effectBytes = serialize(effect);
    const effectSize = effectBytes.length;

    const initAccountBytes = serialize(initAccount);
    const initAccountSize = initAccountBytes.length;

    // console.debug('Effect Bytes: [%s]', Array.of(effectBytes).toString());

    if (buf.length < 1 + initAccountSize + effectSize) {
      throw new Error(
        `WASM memory overflow, buffer length: ${buf.length}, required: ${1 + initAccountSize + effectSize}`
      );
    }

    let offset = 1;
    buf.set(effectBytes, offset);
    offset += effectSize;
    buf.set(initAccountBytes, offset);

    const initState = exports.init_state as Function;
    const newEffectSize: number = initState(effectSize, initAccountSize);
    const data = new Uint8Array(mem.buffer);
    const newEffectBytes = data.slice(1, newEffectSize + 1);
    const newEffect = deserialize(Effect, newEffectBytes);

    if (newEffect.error !== undefined) {
      console.error(newEffect.error);
      throw newEffect.error;
    } else {
      context.applyEffect(newEffect);
    }
  }

  async customHandleEvent(context: GameContext, event: GameEvent) {
    const exports = this.#instance.exports;
    const mem = exports.memory as WebAssembly.Memory;
    let buf = new Uint8Array(mem.buffer);

    const effect = Effect.fromContext(context);
    const effectBytes = serialize(effect);
    const effectSize = effectBytes.length;

    const eventBytes = serialize(event);
    const eventSize = eventBytes.length;

    console.log("Effect:", effect);

    if (buf.length < 1 + eventSize + effectSize) {
      throw new Error(`WASM memory overflow, buffer length: ${buf.length}, required: ${1 + eventSize + effectSize}`);
    }

    let offset = 1;
    buf.set(effectBytes, offset);
    offset += effectSize;
    buf.set(eventBytes, offset);

    const handleEvent = exports.handle_event as Function;
    const newEffectSize: number = handleEvent(effectSize, eventSize);
    switch (newEffectSize) {
      case 0:
        throw (new Error("Serializing effect failed"));
      case 1:
        throw (new Error("Deserializing effect failed"));
      case 2:
        throw (new Error("Deserializing event failed"));
    }
    const data = new Uint8Array(mem.buffer);
    const newEffectBytes = data.slice(1, newEffectSize + 1);

    let newEffect: Effect;
    try {
      newEffect = deserialize(Effect, newEffectBytes);
      console.debug('Return effect: ', newEffect);
    } catch (err: any) {
      console.debug('Failed to deserialize effect, raw: [%s]', Array.from(newEffectBytes).toString());
      throw err;
    }

    if (newEffect.error !== undefined) {
      throw newEffect.error;
    } else {
      context.applyEffect(newEffect);
    }
  }
}
