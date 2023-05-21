import { AttachGameParams, IConnection, SubmitEventParams } from './connection';
import { IEncryptor } from './encryptor';
import { ITransport } from './transport';
import { SecretState } from './secret-state';
import { makeCustomEvent } from './events';
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
  #transport: ITransport;
  #connection: IConnection;
  #gameAddr: string;
  #addr: string;
  #opHist: OpIdent[];
  #secretState: SecretState;

  constructor(addr: string, gameAddr: string, transport: ITransport, encryptor: IEncryptor, connection: IConnection) {
    this.#addr = addr;
    this.#gameAddr = gameAddr;
    this.#transport = transport;
    this.#encryptor = encryptor;
    this.#connection = connection;
    this.#opHist = new Array();
    this.#secretState = new SecretState(encryptor);
  }

  async attachGame(): Promise<void> {
    const key = await this.#encryptor.exportPublicKey(undefined);
    await this.#connection.attachGame(this.#gameAddr,
      new AttachGameParams({
        signer: this.#addr,
        key,
      })
    );
  }

  async submitEvent(event: any): Promise<void> {
    await this.#connection.submitEvent(this.#gameAddr,
      new SubmitEventParams({
        event,
      }));
  }

  async submitCustomEvent(customEvent: any): Promise<void> {
    const event = makeCustomEvent(this.#gameAddr, customEvent);
    await this.#connection.submitEvent(this.#gameAddr,
      new SubmitEventParams({
        event,
      }));
  }

  async handleDecision(ctx: GameContext): Promise<Event[]> {
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
}