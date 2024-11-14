import { IWallet } from './wallet'
import {
  GameAccount,
  GameBundle,
  ServerAccount,
  PlayerProfile,
  VoteType,
  RegistrationAccount,
  INft,
  IToken,
  RegistrationWithGames,
  RecipientAccount,
  EntryType,
  ITokenWithBalance,
  IPlayerProfile,
} from './accounts'
import { IStorage } from './storage'
import { ResponseHandle } from './response'
import { Chain } from './common'

export type RecipientSlotOwnerInit = { addr: string } | { identifier: string }

export type RecipientSlotShareInit = {
  owner: RecipientSlotOwnerInit
  weights: number
}

export type RecipientSlotInit = {
  id: number
  slotType: 'nft' | 'token'
  tokenAddr: string
  initShares: RecipientSlotShareInit[]
}

export type CreateGameAccountParams = {
  title: string
  bundleAddr: string
  tokenAddr: string
  maxPlayers: number
  entryType: EntryType
  registrationAddr: string
  data: Uint8Array
} & ({ recipientAddr: string } | { recipientParams: CreateRecipientParams })

export type CreateGameResponse = {
  gameAddr: string
  signature: string
}

export type CreateGameError = 'invalid-title' | 'invalid-depsoit-range' | CreateRecipientError

export type CloseGameAccountParams = {
  gameAddr: string
}

export type JoinParams = {
  gameAddr: string
  amount: bigint
  position: number
  verifyKey: string
  createProfileIfNeeded?: boolean
}

export type JoinError =
  | 'table-is-full'
  | 'insufficient-funds'
  | 'game-not-served'
  | 'unsupported-entry-type'
  | 'invalid-deposit-amount'
  | 'game-not-found'
  | 'profile-not-found'
  | CreatePlayerProfileError // As we can create profile at first join

export type JoinResponse = {
  signature: string
}

export type DepositParams = {
  playerAddr: string
  gameAddr: string
  amount: bigint
  settleVersion: bigint
}

export type VoteParams = {
  gameAddr: string
  voteType: VoteType
  voterAddr: string
  voteeAddr: string
}

export type CreatePlayerProfileParams = {
  nick: string
  pfp?: string
}

export type CreatePlayerProfileResponse = {
  profile: IPlayerProfile
  signature: string
}

export type CreatePlayerProfileError = 'invalid-nick'

export type PublishGameParams = {
  uri: string
  name: string
  symbol: string
}

export type CreateRegistrationParams = {
  isPrivate: boolean
  size: number
}

export type CreateRecipientParams = {
  capAddr?: string
  slots: RecipientSlotInit[]
}

export type CreateRecipientResponse = {
  recipientAddr: string
  signature: string
}

export type CreateRecipientError = 'duplicated-id' | 'invalid-size'

export type RegisterGameParams = {
  gameAddr: string
  regAddr: string
}

export type RegisterGameResponse = {
  gameAddr: string
  regAddr: string
}

export type RegisterGameError = 'registration-is-full'

export type UnregisterGameParams = {
  gameAddr: string
  regAddr: string
}

export type RecipientClaimParams = {
  recipientAddr: string
}

export type RecipientClaimResponse = {
  recipientAddr: string
  signature: string
}

export type RecipientClaimError = 'not-found'

export interface ITransport {
  get chain(): Chain

  createGameAccount(
    wallet: IWallet,
    params: CreateGameAccountParams,
    resp: ResponseHandle<CreateGameResponse, CreateGameError>
  ): Promise<void>

  closeGameAccount(wallet: IWallet, params: CloseGameAccountParams, resp: ResponseHandle): Promise<void>

  join(wallet: IWallet, params: JoinParams, resp: ResponseHandle<JoinResponse, JoinError>): Promise<void>

  // deposit(wallet: IWallet, params: DepositParams): Promise<TransactionResult<void>>

  // vote(wallet: IWallet, params: VoteParams): Promise<TransactionResult<void>>

  createPlayerProfile(
    wallet: IWallet,
    params: CreatePlayerProfileParams,
    resp: ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>
  ): Promise<void>

  createRecipient(
    wallet: IWallet,
    params: CreateRecipientParams,
    resp: ResponseHandle<CreateRecipientResponse, CreateRecipientError>
  ): Promise<void>

  // publishGame(wallet: IWallet, params: PublishGameParams): Promise<TransactionResult<string>>

  // createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<TransactionResult<string>>

  registerGame(
    wallet: IWallet,
    params: RegisterGameParams,
    resp: ResponseHandle<RegisterGameResponse, RegisterGameError>
  ): Promise<void>

  unregisterGame(wallet: IWallet, params: UnregisterGameParams, resp: ResponseHandle): Promise<void>

  getGameAccount(addr: string): Promise<GameAccount | undefined>

  getGameBundle(addr: string): Promise<GameBundle | undefined>

  getPlayerProfile(addr: string): Promise<PlayerProfile | undefined>

  getServerAccount(addr: string): Promise<ServerAccount | undefined>

  getRegistration(addr: string): Promise<RegistrationAccount | undefined>

  getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined>

  getRecipient(addr: string): Promise<RecipientAccount | undefined>

  getTokenDecimals(addr: string): Promise<number | undefined>

  getToken(addr: string): Promise<IToken | undefined>

  getNft(addr: string): Promise<INft | undefined>

  listTokens(tokenAddrs: string[]): Promise<IToken[]>

  listTokensWithBalance(walletAddr: string, tokenAddrs: string[], storage?: IStorage): Promise<ITokenWithBalance[]>

  listNfts(walletAddr: string): Promise<INft[]>

  recipientClaim(
    wallet: IWallet,
    params: RecipientClaimParams,
    resp: ResponseHandle<RecipientClaimResponse, RecipientClaimError>
  ): Promise<void>
}
