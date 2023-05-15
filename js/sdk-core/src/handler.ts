import { deserialize, serialize } from '@race/borsh';
import { GameAccount, GameBundle, PlayerJoin, ServerJoin } from './accounts';
import { AnswerDecision, GameEvent, GameStart, Leave, Mask, Lock, RandomnessReady, SecretsReady, ShareSecrets, Sync } from './events';
import { GameContext } from './game-context';
import { IEncryptor } from './encryptor';
import { Effect } from './effect';

/**
 * A subset of GameAccount, used in handler initialization.
 */
export interface IInitAccount {
  addr: string;
  players: PlayerJoin[];
  servers: ServerJoin[];
  data: Uint8Array;
  accessVersion: bigint;
  settleVersion: bigint;
}

export class InitAccount {
  readonly addr: string;
  readonly players: PlayerJoin[];
  readonly servers: ServerJoin[];
  readonly data: Uint8Array;
  readonly accessVersion: bigint;
  readonly settleVersion: bigint;
  constructor(fields: IInitAccount) {
    this.addr = fields.addr;
    this.accessVersion = fields.accessVersion;
    this.settleVersion = fields.settleVersion;
    this.data = fields.data;
    this.players = fields.players;
    this.servers = fields.servers;
  }
  static createFromGameAccount(
    gameAccount: GameAccount,
    transactorAccessVersion: bigint,
    transactorSettleVersion: bigint
  ): InitAccount {
    let { addr, players, servers, data } = gameAccount;
    players = players.filter(p => p.accessVersion <= transactorAccessVersion);
    servers = servers.filter(s => s.accessVersion <= transactorAccessVersion);
    return new InitAccount({
      addr,
      data,
      players,
      servers,
      accessVersion: transactorAccessVersion,
      settleVersion: transactorSettleVersion,
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

  constructor(instance: WebAssembly.Instance, encryptor: IEncryptor) {
    this.#encryptor = encryptor;
    this.#instance = instance;
  }

  static async initialize(gameBundle: GameBundle, encryptor: IEncryptor): Promise<Handler> {
    const importObject = {
      imports: {
        memory: new WebAssembly.Memory({
          shared: true,
          maximum: 100,
          initial: 100,
        })
      }
    };
    let initiatedSource;
    if (gameBundle.data.length === 0) {
      initiatedSource = await WebAssembly.instantiateStreaming(fetch(gameBundle.uri), importObject);
    } else {
      initiatedSource = await WebAssembly.instantiate(gameBundle.data, importObject);
    }
    return new Handler(initiatedSource.instance, encryptor);
  }

  async handleEvent(context: GameContext, event: GameEvent) {
    this.generalPreHandleEvent(context, event, this.#encryptor);
    this.customHandleEvent(context, event);
    this.generalPostHandleEvent(context, event);
    context.applyAndTakeSettles();
  }

  async initState(context: GameContext, initAccount: InitAccount) {
    this.generalPreInitState(context, initAccount);
    this.customInitState(context, initAccount);
    this.generalPostInitState(context, initAccount);
  }

  async generalPreInitState(_context: GameContext, _initAccount: InitAccount) { }

  async generalPostInitState(_context: GameContext, _initAccount: InitAccount) { }

  async generalPreHandleEvent(context: GameContext, event: GameEvent, encryptor: IEncryptor) {
    if (event instanceof ShareSecrets) {
      const { sender, shares } = event;
      context.addSharedSecrets(sender, shares);
      if (context.isSecretsReady()) {
        context.dispatchEventInstantly(new SecretsReady());
      }
    } else if (event instanceof AnswerDecision) {
      const { decisionId, ciphertext, sender, digest } = event;
      context.answerDecision(decisionId, sender, ciphertext, digest);
    } else if (event instanceof Mask) {
      const { sender, randomId, ciphertexts } = event;
      context.randomizeAndMask(sender, randomId, ciphertexts);
    } else if (event instanceof Lock) {
      const { sender, randomId, ciphertextsAndDigests } = event;
      context.lock(sender, randomId, ciphertextsAndDigests);
    } else if (event instanceof Sync) {
      const { accessVersion, newPlayers, newServers } = event;
      if (accessVersion < context.accessVersion) {
        throw new Error('Event ignored');
      }
      for (const p of newPlayers) {
        context.addPlayer(p);
      }
      for (const s of newServers) {
        context.addServer(s);
      }
      context.accessVersion = accessVersion;
    } else if (event instanceof Leave) {
      const { playerAddr } = event;
      const exist = context.players.find(p => p.addr === playerAddr);
      if (exist === undefined) {
        throw new Error('Invalid player address');
      } else {
        context.removePlayer(playerAddr);
      }
    } else if (event instanceof GameStart) {
      const { accessVersion } = event;
      context.status = 'running';
      context.setNodeReady(accessVersion);
    } else if (event instanceof SecretsReady) {
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

  async generalPostHandleEvent(context: GameContext, event: GameEvent) { }

  async customInitState(context: GameContext, initAccount: InitAccount) {
    const exports = this.#instance.exports;
    const mem = exports.memory as WebAssembly.Memory;
    let buf = new Uint8Array(mem.buffer);

    const effect = Effect.fromContext(context);
    const effectBytes = serialize(effect);
    const effectSize = effectBytes.length;

    const initAccountBytes = serialize(initAccount);
    const initAccountSize = initAccountBytes.length;

    let offset = 1;
    buf.set(effectBytes, offset);
    offset += effectSize;
    buf.set(initAccountBytes, offset);

    const initState = exports.init_state as Function;
    const newEffectSize: number = initState(effectSize, initAccountSize);
    const data = new Uint8Array(mem.buffer);
    const newEffectBytes = data.slice(1, (newEffectSize + 1));
    const newEffect = deserialize(Effect, newEffectBytes);

    if (newEffect.error !== undefined) {
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

    let offset = 1;
    buf.set(effectBytes, offset);
    offset += effectSize;
    buf.set(eventBytes, offset);

    const handleEvent = exports.handle_event as Function;
    const newEffectSize: number = handleEvent(effectSize, eventSize);
    const data = new Uint8Array(mem.buffer);
    const newEffectBytes = data.slice(1, (newEffectSize + 1));
    const newEffect = deserialize(Effect, newEffectBytes);

    if (newEffect.error !== undefined) {
      throw newEffect.error;
    } else {
      context.applyEffect(newEffect);
    }
  }
}
