import { AttachGameParams, IConnection, SubmitEventParams } from './connection';
import { IEncryptor } from './encryptor';
import { SecretState } from './secret-state';
import { GameContext } from './game-context';
import { Id } from './types';

type OpIdent =
  | {
      kind: 'random-secret';
      randomId: Id;
      toAddr: string | undefined;
      index: number;
    }
  | {
      kind: 'answer-secret';
      decisionId: Id;
    }
  | {
      kind: 'lock';
      randomId: Id;
    }
  | {
      kind: 'mask';
      randomId: Id;
    };

export class Client {
  #encryptor: IEncryptor;
  #connection: IConnection;
  #addr: string;
  #opHist: OpIdent[];
  #secretState: SecretState;

  constructor(addr: string, encryptor: IEncryptor, connection: IConnection) {
    this.#addr = addr;
    this.#encryptor = encryptor;
    this.#connection = connection;
    this.#opHist = new Array();
    this.#secretState = new SecretState(encryptor);
  }

  async attachGame(): Promise<void> {
    const key = await this.#encryptor.exportPublicKey(undefined);
    await this.#connection.attachGame(
      new AttachGameParams({
        signer: this.#addr,
        key,
      })
    );
  }

  async submitEvent(event: any): Promise<void> {
    await this.#connection.submitEvent(
      new SubmitEventParams({
        event,
      })
    );
  }

  async handleDecision(_ctx: GameContext): Promise<Event[]> {
    return [];
  }

  loadRandomStates(ctx: GameContext) {
    for (let randomState of ctx.randomStates) {
      if (!this.#secretState.isRandomLoaded(randomState.id)) {
        this.#secretState.genRandomStates(randomState.id, randomState.size);
      }
    }
  }

  async handleUpdatedContext(ctx: GameContext): Promise<Event[]> {
    this.loadRandomStates(ctx);
    const events = await this.handleDecision(ctx);
    return events;
  }

  flushSecretStates() {
    this.#secretState.clear();
    this.#opHist.splice(0);
  }

  async decrypt(ctx: GameContext, randomId: Id): Promise<Map<number, string>> {
    let randomState = ctx.getRandomState(randomId);
    let options = randomState.options;
    let revealed = await this.#encryptor.decryptWithSecrets(
      randomState.listRevealedCiphertexts(),
      randomState.listRevealedSecrets(),
      options
    );
    let assigned = await this.#encryptor.decryptWithSecrets(
      randomState.listAssignedCiphertexts(this.#addr),
      randomState.listSharedSecrets(this.#addr),
      options
    );

    return new Map([...revealed, ...assigned]);
  }
}
