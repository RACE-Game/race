import { AttachGameParams, AttachResponse, IConnection, SubmitEventParams } from './connection'
import { IEncryptor } from './encryptor'
import { SecretState } from './secret-state'
import { GameContext } from './game-context'

type OpIdent =
    | {
          kind: 'random-secret'
          randomId: number
          toAddr: string | undefined
          index: number
      }
    | {
          kind: 'answer-secret'
          decisionId: number
      }
    | {
          kind: 'lock'
          randomId: number
      }
    | {
          kind: 'mask'
          randomId: number
      }

export class Client {
    __encryptor: IEncryptor
    __connection: IConnection
    __addr: string
    __opHist: OpIdent[]
    __secretState: SecretState

    constructor(addr: string, encryptor: IEncryptor, connection: IConnection) {
        this.__addr = addr
        this.__encryptor = encryptor
        this.__connection = connection
        this.__opHist = new Array()
        this.__secretState = new SecretState(encryptor)
    }

    async attachGame(): Promise<AttachResponse> {
        const key = await this.__encryptor.exportPublicKey(undefined)
        return await this.__connection.attachGame(
            new AttachGameParams({
                signer: this.__addr,
                key,
            })
        )
    }

    async submitEvent(event: any): Promise<void> {
        await this.__connection.submitEvent(
            new SubmitEventParams({
                event,
            })
        )
    }

    async handleDecision(_ctx: GameContext): Promise<Event[]> {
        return []
    }

    loadRandomStates(ctx: GameContext) {
        for (let randomState of ctx.randomStates) {
            if (!this.__secretState.isRandomLoaded(randomState.id)) {
                this.__secretState.genRandomStates(randomState.id, randomState.size)
            }
        }
    }

    async handleUpdatedContext(ctx: GameContext): Promise<Event[]> {
        this.loadRandomStates(ctx)
        const events = await this.handleDecision(ctx)
        return events
    }

    flushSecretStates() {
        this.__secretState.clear()
        this.__opHist.splice(0)
    }

    async decrypt(ctx: GameContext, randomId: number): Promise<Map<number, string>> {
        let randomState = ctx.getRandomState(randomId)
        let options = randomState.options
        let revealed = await this.__encryptor.decryptWithSecrets(
            randomState.listRevealedCiphertexts(),
            randomState.listRevealedSecrets(),
            options
        )
        let assigned = await this.__encryptor.decryptWithSecrets(
            randomState.listAssignedCiphertexts(this.__addr),
            randomState.listSharedSecrets(this.__addr),
            options
        )

        return new Map([...revealed, ...assigned])
    }
}
