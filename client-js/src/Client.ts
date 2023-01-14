
export class Client {
  _encryptor: IEncryptor
  _transport: ITransport
  readonly addr: string
  _secretShares: SecretState[]
}
