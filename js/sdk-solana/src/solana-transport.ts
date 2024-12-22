import {
    SystemProgram,
    Connection,
    PublicKey,
    Keypair,
    ComputeBudgetProgram,
    TransactionMessage,
    TransactionInstruction,
    VersionedTransaction,
    AccountInfo,
    TransactionSignature,
    Commitment,
} from '@solana/web3.js'
import { Buffer } from 'buffer'
import {
    AccountLayout,
    MintLayout,
    NATIVE_MINT,
    TOKEN_PROGRAM_ID,
    createInitializeAccountInstruction,
    createTransferInstruction,
    getAssociatedTokenAddressSync,
    getMint,
} from '@solana/spl-token'
import {
    IWallet,
    ITransport,
    CreateGameAccountParams,
    CloseGameAccountParams,
    JoinParams,
    DepositParams,
    VoteParams,
    CreatePlayerProfileParams,
    PublishGameParams,
    CreateRegistrationParams,
    RegisterGameParams,
    UnregisterGameParams,
    GameAccount,
    GameBundle,
    PlayerProfile,
    ServerAccount,
    RegistrationAccount,
    Token,
    Nft,
    RegistrationWithGames,
    RecipientAccount,
    RecipientSlot,
    RecipientClaimParams,
    TokenWithBalance,
    ResponseHandle,
    CreateGameResponse,
    CreateGameError,
    JoinError,
    RecipientClaimResponse,
    RecipientClaimError,
    CreatePlayerProfileError,
    Result,
    JoinResponse,
    CreatePlayerProfileResponse,
    SendTransactionResult,
    CreateRecipientResponse,
    CreateRecipientError,
    CreateRecipientParams,
    DepositResponse,
    DepositError,
    AttachBonusParams,
    AttachBonusResponse,
    AttachBonusError,
    CloseGameAccountResponse,
    CloseGameAccountError,
} from '@race-foundation/sdk-core'
import * as instruction from './instruction'

import {
    GAME_ACCOUNT_LEN,
    NAME_LEN,
    PROFILE_ACCOUNT_LEN,
    PLAYER_PROFILE_SEED,
    SERVER_PROFILE_SEED,
    RECIPIENT_ACCOUNT_LEN,
} from './constants'

import {
    EntryTypeCash,
    EntryTypeTicket,
    GameState,
    PlayerState,
    RecipientSlotOwnerAssigned,
    RecipientSlotOwnerUnassigned,
    RecipientState,
    RegistryState,
    ServerState,
} from './accounts'

import { PROGRAM_ID, METAPLEX_PROGRAM_ID } from './constants'
import { Metadata } from './metadata'
import { Chain } from '@race-foundation/sdk-core/lib/types/common'
import { SolanaWalletAdapter } from './solana-wallet'

function trimString(s: string): string {
    return s.replace(/\0/g, '')
}

type LegacyToken = {
    name: string
    symbol: string
    logoURI: string
    address: string
    decimals: number
}

const SOL_TOKEN = {
    addr: 'So11111111111111111111111111111111111111112',
    name: 'SOL',
    symbol: 'SOL',
    icon: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png',
    decimals: 9,
}

type SendTransactionOptions = {
    signers?: Keypair[]
    commitment?: Commitment
}

export class SolanaTransport implements ITransport {
    #conn: Connection
    #legacyTokens?: LegacyToken[]

    constructor(endpoint: string) {
        this.#conn = new Connection(endpoint, 'confirmed')
    }

    get chain(): Chain {
        return 'solana'
    }

    async _fetchLegacyTokens() {
        const resp = await fetch('https://arweave.net/60i6lMrqKZU8MtGM27WNrqr3s52ry2munrwMOK4jaO8')
        const m = await resp.json()
        this.#legacyTokens = m['tokens']
    }

    async createGameAccount(
        wallet: IWallet,
        params: CreateGameAccountParams,
        response: ResponseHandle<CreateGameResponse, CreateGameError>
    ): Promise<void> {
        const conn = this.#conn
        const { title, bundleAddr, tokenAddr } = params
        if (title.length > NAME_LEN) {
            return response.failed('invalid-title')
        }

        const payerKey = new PublicKey(wallet.walletAddr)

        let recipientAccountKey: PublicKey
        if ('recipientAddr' in params) {
            recipientAccountKey = new PublicKey(params.recipientAddr)
        } else {
            const createRecipient = await this._prepareCreateRecipient(payerKey, params.recipientParams)
            if ('err' in createRecipient) {
                return response.failed(createRecipient.err)
            }
            let { ixs: createRecipientIxs, recipientAccount, signers } = createRecipient.ok
            recipientAccountKey = recipientAccount.publicKey

            const tx = await makeTransaction(this.#conn, payerKey, createRecipientIxs)

            if ('err' in tx) {
                return response.retryRequired(tx.err)
            }

            const prioritizationFee = await this._getPrioritizationFee([
                recipientAccount.publicKey,
                ...signers.map(k => k.publicKey),
            ])
            createRecipientIxs = [
                ComputeBudgetProgram.setComputeUnitPrice({ microLamports: prioritizationFee }),
                ...createRecipientIxs,
            ]

            tx.ok.sign(signers)
            const sig = await sendTransaction(wallet, tx.ok, conn, response)
            if ('err' in sig) {
                return response.transactionFailed(sig.err)
            } else {
                response.preparing()
            }
        }

        const registrationAccountKey = new PublicKey(params.registrationAddr)

        let ixs: TransactionInstruction[] = []
        let signers: Keypair[] = []

        const { ixs: createGameAccountIxs, account: gameAccount } = await this._prepareCreateAccount(
            payerKey,
            GAME_ACCOUNT_LEN,
            PROGRAM_ID
        )
        ixs.push(...createGameAccountIxs)
        signers.push(gameAccount)

        const tokenMintKey = new PublicKey(tokenAddr)

        let stakeAccountKey
        if (tokenMintKey.equals(NATIVE_MINT)) {
            // For SOL game, use PDA as stake account
            const [pda, _] = PublicKey.findProgramAddressSync([gameAccount.publicKey.toBuffer()], PROGRAM_ID)
            stakeAccountKey = pda
            console.info('Game uses SOL as token, use PDA as stake account:', stakeAccountKey.toBase58())
        } else {
            // For SPL game, use dedicated stake account
            const { tokenAccount: stakeAccount, ixs: createStakeAccountIxs } = await this._prepareCreateTokenAccount(
                payerKey,
                tokenMintKey
            )
            signers.push(stakeAccount)
            ixs.push(...createStakeAccountIxs)
            stakeAccountKey = stakeAccount.publicKey
            console.info('Game uses SPL as token, use dedicated stake account:', stakeAccountKey.toBase58())
        }

        const bundleKey = new PublicKey(bundleAddr)
        const createGame = instruction.createGameAccount({
            ownerKey: payerKey,
            gameAccountKey: gameAccount.publicKey,
            stakeAccountKey,
            recipientAccountKey: recipientAccountKey,
            mint: tokenMintKey,
            gameBundleKey: bundleKey,
            title: title,
            maxPlayers: params.maxPlayers,
            entryType: params.entryType,
            data: params.data,
        })
        console.info('Transaction Instruction[CreateGame]:', createGame)
        ixs.push(createGame)

        const registerGame = instruction.registerGame({
            ownerKey: payerKey,
            gameAccountKey: gameAccount.publicKey,
            registrationAccountKey,
        })

        console.info('Transaction Instruction[RegisterGame]:', registerGame)
        ixs.push(registerGame)

        const tx = await makeTransaction(this.#conn, payerKey, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }

        const sig = await sendTransaction(wallet, tx.ok, conn, response, { signers })
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ gameAddr: gameAccount.publicKey.toBase58(), signature: sig.ok })
        }
    }

    async closeGameAccount(
        wallet: IWallet,
        params: CloseGameAccountParams,
        response: ResponseHandle<CloseGameAccountResponse, CloseGameAccountError>
    ): Promise<void> {
        const conn = this.#conn
        const { gameAddr, regAddr } = params

        const payerKey = new PublicKey(wallet.walletAddr)
        const gameAccountKey = new PublicKey(gameAddr)
        const regAccountKey = new PublicKey(regAddr)

        const gameState = await this._getGameState(gameAccountKey)

        if (gameState === undefined) {
            return response.failed('game-not-found')
        }

        if (!gameState.ownerKey.equals(payerKey)) {
            return response.failed('permission-denied')
        }
        const regState = await this._getRegState(regAccountKey)

        if (regState === undefined) {
            return response.failed('reg-not-found')
        }

        if (regState.games.find(g => g.gameKey.equals(gameAccountKey)) === undefined) {
            return response.failed('game-not-in-reg')
        }
        const ixs = []
        const [pda, _] = PublicKey.findProgramAddressSync([gameAccountKey.toBuffer()], PROGRAM_ID)
        const ata = getAssociatedTokenAddressSync(gameState.tokenKey, payerKey)
        const prioritizationFee = await this._getPrioritizationFee([
            gameAccountKey,
            gameState.stakeKey,
            ata,
            pda,
            regAccountKey,
        ])
        ixs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: prioritizationFee }))
        const unregisterGameIx = new TransactionInstruction({
            keys: [
                {
                    pubkey: payerKey,
                    isSigner: true,
                    isWritable: false,
                },
                {
                    pubkey: regAccountKey,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: gameAccountKey,
                    isSigner: false,
                    isWritable: false,
                },
            ],
            data: Buffer.from(Uint8Array.of(instruction.Instruction.UnregisterGame)),
            programId: PROGRAM_ID,
        })
        ixs.push(unregisterGameIx)
        const closeGameAccountIx = new TransactionInstruction({
            keys: [
                {
                    pubkey: payerKey,
                    isSigner: true,
                    isWritable: false,
                },
                {
                    pubkey: gameAccountKey,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: gameState.stakeKey,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: pda,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: ata,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: TOKEN_PROGRAM_ID,
                    isSigner: false,
                    isWritable: false,
                },
            ],
            data: Buffer.from(Uint8Array.of(instruction.Instruction.CloseGameAccount)),
            programId: PROGRAM_ID,
        })
        ixs.push(closeGameAccountIx)
        const tx = await makeTransaction(this.#conn, payerKey, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response, { commitment: 'confirmed' })
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ signature: sig.ok })
        }
    }

    async join(wallet: IWallet, params: JoinParams, response: ResponseHandle<JoinResponse, JoinError>): Promise<void> {
        const { gameAddr, amount: amountRaw, position, verifyKey } = params
        const gameAccountKey = new PublicKey(gameAddr)
        const playerKey = new PublicKey(wallet.walletAddr)

        // Call RPC functions in Parallel
        const d = new Date()
        const [prioritizationFee, gameState, playerProfile] = await Promise.all([
            this._getPrioritizationFee([gameAccountKey]),
            this._getGameState(gameAccountKey),
            this.getPlayerProfile(wallet.walletAddr),
        ])
        console.debug('Batched RPC calls took %s milliseconds', new Date().getTime() - d.getTime())

        const profileKey0 = playerProfile !== undefined ? new PublicKey(playerProfile?.addr) : undefined

        if (gameState === undefined) return response.failed('game-not-found')

        const accessVersion = gameState.accessVersion
        const settleVersion = gameState.settleVersion

        const mintKey = gameState.tokenKey
        const isWsol = mintKey.equals(NATIVE_MINT)
        const amount = BigInt(amountRaw)

        if (gameState.entryType instanceof EntryTypeCash) {
            if (amount < gameState.entryType.minDeposit || amount > gameState.entryType.maxDeposit) {
                console.warn(
                    `Invalid deposit, maximum = ${gameState.entryType.maxDeposit}, minimum = ${gameState.entryType.minDeposit}, submitted = ${amount}`
                )
                return response.failed('invalid-deposit-amount')
            }
        } else if (gameState.entryType instanceof EntryTypeTicket) {
            if (amount !== gameState.entryType.amount) {
                console.warn(`Invalid deposit, ticket = ${gameState.entryType.amount}, submitted = ${amount}`)
                return response.failed('invalid-deposit-amount')
            }
        } else {
            return response.failed('unsupported-entry-type')
        }

        const stakeAccountKey = gameState.stakeKey

        let ixs = []

        ixs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: prioritizationFee }))

        let profileKey: PublicKey
        if (profileKey0 !== undefined) {
            profileKey = profileKey0
        } else if (params.createProfileIfNeeded) {
            const createProfile = await this._prepareCreatePlayerProfile(playerKey, {
                nick: wallet.walletAddr.substring(0, 6),
            })
            if ('err' in createProfile) {
                return response.failed(createProfile.err)
            }
            const { ixs: createProfileIxs, profileKey: pk } = createProfile.ok
            ixs.push(...createProfileIxs)
            profileKey = pk
        } else {
            return response.failed('profile-not-found')
        }
        let tempAccount
        if (isWsol) {
            const account = Keypair.generate()
            const accountKey = account.publicKey
            const ix = SystemProgram.createAccount({
                fromPubkey: playerKey,
                newAccountPubkey: accountKey,
                lamports: Number(amount),
                space: 0,
                programId: PROGRAM_ID,
            })
            ixs.push(ix)
            tempAccount = account
        } else {
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                playerKey,
                mintKey
            )
            ixs.push(...createTempAccountIxs)
            const playerAta = getAssociatedTokenAddressSync(mintKey, playerKey)
            const transferIx = createTransferInstruction(playerAta, tokenAccount.publicKey, playerKey, amount)
            ixs.push(transferIx)
            tempAccount = tokenAccount
        }
        const joinGameIx = instruction.join({
            playerKey,
            profileKey,
            paymentKey: tempAccount.publicKey,
            gameAccountKey,
            mint: mintKey,
            stakeAccountKey,
            recipientAccountKey: gameState.recipientAddr,
            amount,
            accessVersion,
            settleVersion,
            position,
            verifyKey,
        })
        console.info('Transaction Instruction[Join]:', joinGameIx)
        ixs.push(joinGameIx)
        const tx = await makeTransaction(this.#conn, playerKey, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }
        tx.ok.sign([tempAccount])
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response, { commitment: 'confirmed' })
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ signature: sig.ok })
        }
    }
    async deposit(
        wallet: IWallet,
        params: DepositParams,
        response: ResponseHandle<DepositResponse, DepositError>
    ): Promise<void> {
        const playerKey = new PublicKey(wallet.walletAddr)
        const gameAccountKey = new PublicKey(params.gameAddr)
        // Call RPC functions in Parallel
        const [prioritizationFee, gameState, playerProfile] = await Promise.all([
            this._getPrioritizationFee([gameAccountKey]),
            this._getGameState(gameAccountKey),
            this.getPlayerProfile(wallet.walletAddr),
        ])

        if (gameState === undefined) {
            return response.failed('game-not-found')
        }
        let profileKey
        if (playerProfile === undefined) {
            return response.failed('profile-not-found')
        } else {
            profileKey = new PublicKey(playerProfile.addr)
        }
        if (gameState.transactorKey === undefined) {
            return response.failed('game-not-served')
        }
        const settleVersion = gameState.settleVersion
        const mintKey = gameState.tokenKey
        const isWsol = mintKey.equals(NATIVE_MINT)
        const amount = BigInt(params.amount)
        if (gameState.entryType instanceof EntryTypeCash) {
            if (amount < gameState.entryType.minDeposit || amount > gameState.entryType.maxDeposit) {
                console.warn(
                    `Invalid deposit, maximum = ${gameState.entryType.maxDeposit}, minimum = ${gameState.entryType.minDeposit}, submitted = ${amount}`
                )
                return response.failed('invalid-deposit-amount')
            }
        } else if (gameState.entryType instanceof EntryTypeTicket) {
            if (amount !== gameState.entryType.amount) {
                console.warn(`Invalid deposit, ticket = ${gameState.entryType.amount}, submitted = ${amount}`)
                return response.failed('invalid-deposit-amount')
            }
        } else {
            return response.failed('unsupported-entry-type')
        }
        let ixs = []
        ixs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: prioritizationFee }))

        let tempAccount
        if (isWsol) {
            const account = Keypair.generate()
            const accountKey = account.publicKey
            const ix = SystemProgram.createAccount({
                fromPubkey: playerKey,
                newAccountPubkey: accountKey,
                lamports: Number(amount),
                space: 0,
                programId: PROGRAM_ID,
            })
            ixs.push(ix)
            tempAccount = account
        } else {
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                playerKey,
                mintKey
            )
            ixs.push(...createTempAccountIxs)
            const playerAta = getAssociatedTokenAddressSync(mintKey, playerKey)
            const transferIx = createTransferInstruction(playerAta, tokenAccount.publicKey, playerKey, amount)
            ixs.push(transferIx)
            tempAccount = tokenAccount
        }
        const depositGameIx = instruction.deposit({
            playerKey,
            profileKey,
            paymentKey: tempAccount.publicKey,
            gameAccountKey,
            mint: mintKey,
            stakeAccountKey: gameState.stakeKey,
            recipientAccountKey: gameState.recipientAddr,
            amount,
            settleVersion,
        })
        console.info('Transaction Instruction[Deposit]:', depositGameIx)
        ixs.push(depositGameIx)
        const tx = await makeTransaction(this.#conn, playerKey, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }
        tx.ok.sign([tempAccount])
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response, { commitment: 'confirmed' })
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ signature: sig.ok })
        }
    }

    async attachBonus(
        wallet: IWallet,
        params: AttachBonusParams,
        response: ResponseHandle<AttachBonusResponse, AttachBonusError>
    ): Promise<void> {
        const payerKey = new PublicKey(wallet.walletAddr)
        const gameAccountKey = new PublicKey(params.gameAddr)
        const gameState = await this._getGameState(gameAccountKey)
        if (gameState === undefined) {
            return response.failed('game-not-found')
        }
        let ixs = []
        let tempAccountKeys = []
        let signers = []

        for (const bonus of params.bonuses) {
            const { tokenAddr, amount } = bonus
            const mintKey = new PublicKey(tokenAddr)
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                payerKey,
                mintKey
            )
            ixs.push(...createTempAccountIxs)
            const payerAta = getAssociatedTokenAddressSync(mintKey, payerKey)
            const transferIx = createTransferInstruction(payerAta, tokenAccount.publicKey, payerKey, amount)
            ixs.push(transferIx)
            tempAccountKeys.push(tokenAccount.publicKey)
            signers.push(tokenAccount)
        }

        const attachBonusIx = instruction.attachBonus({
            payerKey,
            gameAccountKey: new PublicKey(params.gameAddr),
            stakeAccountKey: gameState.stakeKey,
            identifiers: params.bonuses.map(b => b.identifier),
            tempAccountKeys,
        })

        if ('err' in attachBonusIx) {
            return response.failed(attachBonusIx.err)
        }
        console.info('Transaction Instruction[attachBonus]:', attachBonusIx.ok)
        ixs.push(attachBonusIx.ok)
        const tx = await makeTransaction(this.#conn, payerKey, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response, { signers })
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ signature: sig.ok })
        }
    }

    async publishGame(_wallet: IWallet, _params: PublishGameParams): Promise<void> {
        throw new Error('unimplemented')
    }
    async vote(_wallet: IWallet, _params: VoteParams): Promise<void> {
        throw new Error('unimplemented')
    }
    async recipientClaim(
        wallet: IWallet,
        params: RecipientClaimParams,
        response: ResponseHandle<RecipientClaimResponse, RecipientClaimError>
    ): Promise<void> {
        const payerKey = new PublicKey(wallet.walletAddr)
        const recipientKey = new PublicKey(params.recipientAddr)
        const recipientState = await this._getRecipientState(recipientKey)
        if (recipientState === undefined) {
            return response.failed('not-found')
        }
        const recipientClaimIx = instruction.claim({
            recipientKey,
            payerKey,
            recipientState,
        })
        if ('err' in recipientClaimIx) {
            return response.failed(recipientClaimIx.err)
        }
        const tx = await makeTransaction(this.#conn, payerKey, [recipientClaimIx.ok])
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response)
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ recipientAddr: params.recipientAddr, signature: sig.ok })
        }
    }
    async _prepareCreatePlayerProfile(
        payerKey: PublicKey,
        params: CreatePlayerProfileParams
    ): Promise<Result<{ ixs: TransactionInstruction[]; profileKey: PublicKey }, CreatePlayerProfileError>> {
        let ixs = []
        const { nick, pfp } = params
        if (nick.length > 16) {
            return { err: 'invalid-nick' }
        }
        console.info('Payer Public Key:', payerKey.toBase58())
        const profileKey = await PublicKey.createWithSeed(payerKey, PLAYER_PROFILE_SEED, PROGRAM_ID)
        console.info('Player profile public key: ', profileKey.toBase58())
        if (!(await this.#conn.getAccountInfo(profileKey))) {
            const lamports = await this.#conn.getMinimumBalanceForRentExemption(PROFILE_ACCOUNT_LEN)
            const ix = SystemProgram.createAccountWithSeed({
                fromPubkey: payerKey,
                basePubkey: payerKey,
                newAccountPubkey: profileKey,
                lamports: lamports,
                space: PROFILE_ACCOUNT_LEN,
                seed: PLAYER_PROFILE_SEED,
                programId: PROGRAM_ID,
            })
            console.info('Transaction Instruction[CreateAccount]:', ix)
            ixs.push(ix)
        }
        const pfpKey = !pfp ? PublicKey.default : new PublicKey(pfp)
        const createProfile = instruction.createPlayerProfile(payerKey, profileKey, nick, pfpKey)
        console.info('Transaction Instruction[CreatePlayerProfile]:', createProfile)
        ixs.push(createProfile)
        return {
            ok: {
                ixs,
                profileKey,
            },
        }
    }
    async createPlayerProfile(
        wallet: IWallet,
        params: CreatePlayerProfileParams,
        response: ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>
    ): Promise<void> {
        let ixs: TransactionInstruction[] = []
        const payerKey = new PublicKey(wallet.walletAddr)
        const createPlayerProfile = await this._prepareCreatePlayerProfile(payerKey, params)
        if ('err' in createPlayerProfile) {
            return response.failed(createPlayerProfile.err)
        }
        const { ixs: createProfileIxs, profileKey } = createPlayerProfile.ok
        ixs.push(...createProfileIxs)
        let tx = await makeTransaction(this.#conn, payerKey, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(wallet, tx.ok, this.#conn, response)
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({
                signature: sig.ok,
                profile: {
                    nick: params.nick,
                    pfp: params.pfp,
                    addr: profileKey.toBase58(),
                },
            })
        }
    }
    async _prepareCreateTokenAccount(
        payerKey: PublicKey,
        tokenMintKey: PublicKey
    ): Promise<{ ixs: TransactionInstruction[]; tokenAccount: Keypair }> {
        const { account, ixs } = await this._prepareCreateAccount(payerKey, AccountLayout.span, TOKEN_PROGRAM_ID)
        const initStakeAccountIx = createInitializeAccountInstruction(
            account.publicKey,
            tokenMintKey,
            payerKey,
            TOKEN_PROGRAM_ID
        )
        console.info('Transaction Instruction[InitializeAccount]:', initStakeAccountIx)
        ixs.push(initStakeAccountIx)
        return {
            ixs,
            tokenAccount: account,
        }
    }
    async _prepareCreateAccount(
        payerKey: PublicKey,
        size: number,
        programId: PublicKey
    ): Promise<{ ixs: TransactionInstruction[]; account: Keypair }> {
        const account = Keypair.generate()
        const accountKey = account.publicKey
        const lamports = await this.#conn.getMinimumBalanceForRentExemption(size)
        const ix = SystemProgram.createAccount({
            fromPubkey: payerKey,
            newAccountPubkey: accountKey,
            lamports: lamports,
            space: size,
            programId,
        })
        console.info('Transaction Instruction[CreateAccount]:', ix)
        return { ixs: [ix], account }
    }
    async _prepareCreateRecipient(
        payerKey: PublicKey,
        params: CreateRecipientParams
    ): Promise<
        Result<{ recipientAccount: Keypair; ixs: TransactionInstruction[]; signers: Keypair[] }, CreateRecipientError>
    > {
        if (params.slots.length > 10) {
            return { err: 'invalid-size' }
        }
        let ixs: TransactionInstruction[] = []
        let signers: Keypair[] = []
        let capKey: PublicKey
        if (params.capAddr === undefined) {
            capKey = payerKey
        } else {
            capKey = new PublicKey(params.capAddr)
        }
        // Create Recipient Account
        let { ixs: createRecipientAccountIxs, account: recipientAccount } = await this._prepareCreateAccount(
            payerKey,
            RECIPIENT_ACCOUNT_LEN,
            PROGRAM_ID
        )
        ixs.push(...createRecipientAccountIxs)
        signers.push(recipientAccount)
        // Create Slot Stake Accounts
        let usedId: number[] = []
        let slots: instruction.SlotInit[] = []
        for (const slot of params.slots) {
            // Don't allow duplicated slot id
            if (usedId.includes(slot.id)) {
                return { err: 'duplicated-id' }
            } else {
                usedId.push(slot.id)
            }
            let stakeAddr: PublicKey
            if (slot.tokenAddr === NATIVE_MINT.toBase58()) {
                // Use PDA as stake account for SOL slot
                const [pda, _] = PublicKey.findProgramAddressSync(
                    [recipientAccount.publicKey.toBytes(), Uint8Array.of(slot.id)],
                    PROGRAM_ID
                )
                stakeAddr = pda
            } else {
                // Use dedicated stake account
                const { ixs: createStakeAccountIxs, tokenAccount: stakeAccount } =
                    await this._prepareCreateTokenAccount(payerKey, new PublicKey(slot.tokenAddr))
                ixs.push(...createStakeAccountIxs)
                signers.push(stakeAccount)
                stakeAddr = stakeAccount.publicKey
            }
            const slotInit = new instruction.SlotInit({
                id: slot.id,
                tokenAddr: new PublicKey(slot.tokenAddr),
                stakeAddr,
                slotType: slot.slotType === 'token' ? 0 : 1,
                initShares: slot.initShares.map(share => {
                    let owner
                    if ('addr' in share.owner) {
                        owner = new RecipientSlotOwnerAssigned({ addr: share.owner.addr })
                    } else {
                        owner = new RecipientSlotOwnerUnassigned({ identifier: share.owner.identifier })
                    }
                    return new instruction.SlotShareInit({
                        owner,
                        weights: share.weights,
                    })
                }),
            })
            slots.push(slotInit)
        }
        console.debug('Slots for recipient:', slots)
        // Initialize Recipient Account
        const createRecipientIx = instruction.createRecipient({
            payerKey,
            recipientKey: recipientAccount.publicKey,
            slots,
            capKey,
        })
        ixs.push(createRecipientIx)
        return {
            ok: {
                ixs,
                recipientAccount,
                signers,
            },
        }
    }
    async createRecipient(
        wallet: IWallet,
        params: CreateRecipientParams,
        response: ResponseHandle<CreateRecipientResponse, CreateRecipientError>
    ): Promise<void> {
        const payerKey = new PublicKey(wallet.walletAddr)
        const createRecipient = await this._prepareCreateRecipient(payerKey, params)
        if ('err' in createRecipient) {
            return response.failed(createRecipient.err)
        }
        const { ixs, recipientAccount, signers } = createRecipient.ok
        const tx = await makeTransaction(this.#conn, payerKey, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const transaction = tx.ok
        transaction.sign(signers)
        const sig = await sendTransaction(wallet, transaction, this.#conn, response)
        if ('err' in sig) {
            response.transactionFailed(sig.err)
        } else {
            response.succeed({ recipientAddr: recipientAccount.publicKey.toBase58(), signature: sig.ok })
        }
    }
    async createRegistration(_wallet: IWallet, _params: CreateRegistrationParams): Promise<void> {
        throw new Error('unimplemented')
    }
    async registerGame(_wallet: IWallet, _params: RegisterGameParams): Promise<void> {
        throw new Error('unimplemented')
    }
    async unregisterGame(_wallet: IWallet, _params: UnregisterGameParams): Promise<void> {
        throw new Error('unimplemented')
    }
    async getGameAccount(addr: string): Promise<GameAccount | undefined> {
        const gameAccountKey = new PublicKey(addr)
        const gameState = await this._getGameState(gameAccountKey)
        if (gameState !== undefined) {
            return gameState.generalize(new PublicKey(addr))
        } else {
            return undefined
        }
    }
    async getGameBundle(addr: string): Promise<GameBundle | undefined> {
        const mintKey = new PublicKey(addr)
        const [metadataKey] = PublicKey.findProgramAddressSync(
            [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
            METAPLEX_PROGRAM_ID
        )
        const metadataAccount = await this.#conn.getAccountInfo(metadataKey)
        if (metadataAccount === null) {
            return undefined
        }
        const metadataState = Metadata.deserialize(metadataAccount.data)
        console.debug('Metadata of game bundle:', metadataState)
        let { uri, name } = metadataState.data
        // URI should contains the wasm property
        let resp = await fetch(trimString(uri))
        let json = await resp.json()
        let files: any[] = json['properties']['files']
        let wasm_file = files.find(f => f['type'] == 'application/wasm')
        return { addr, uri: wasm_file['uri'], name: trimString(name), data: new Uint8Array(0) }
    }
    async getPlayerProfile(addr: string): Promise<PlayerProfile | undefined> {
        const conn = this.#conn
        const playerKey = new PublicKey(addr)
        const profileKey = await PublicKey.createWithSeed(playerKey, PLAYER_PROFILE_SEED, PROGRAM_ID)
        const profileAccount = await conn.getAccountInfo(profileKey)
        if (profileAccount) {
            const profileAccountData = profileAccount.data
            const state = PlayerState.deserialize(profileAccountData)
            return state.generalize(playerKey)
        } else {
            return undefined
        }
    }
    async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
        const serverKey = new PublicKey(addr)
        const serverState = await this._getServerState(serverKey)
        if (serverState !== undefined) {
            return serverState.generalize()
        } else {
            return undefined
        }
    }
    async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
        const regKey = new PublicKey(addr)
        const regState = await this._getRegState(regKey)
        if (regState !== undefined) {
            return regState.generalize(regKey)
        } else {
            return undefined
        }
    }
    async getRegistrationWithGames(addr: string): Promise<RegistrationWithGames | undefined> {
        const regAccount = await this.getRegistration(addr)
        if (regAccount === undefined) return undefined
        const keys = regAccount.games.map(g => new PublicKey(g.addr))
        const gameStates = await this._getMultiGameStates(keys)
        let games: Array<GameAccount> = []
        for (let i = 0; i < gameStates.length; i++) {
            const gs = gameStates[i]
            if (gs !== undefined) {
                games.push(gs.generalize(keys[i]))
            }
        }
        return {
            ...regAccount,
            games,
        }
    }
    async getRecipient(addr: string): Promise<RecipientAccount | undefined> {
        const recipientKey = new PublicKey(addr)
        const recipientState = await this._getRecipientState(recipientKey)
        if (recipientState === undefined) return undefined
        let slots: RecipientSlot[] = []
        for (const slot of recipientState.slots) {
            let balance
            if (slot.tokenAddr.equals(NATIVE_MINT)) {
                const resp = await this.#conn.getAccountInfo(slot.stakeAddr)
                balance = BigInt(resp?.lamports || 0n)
            } else {
                const resp = await this.#conn.getTokenAccountBalance(slot.stakeAddr)
                balance = BigInt(resp.value.amount)
            }
            slots.push(slot.generalize(balance))
        }
        return recipientState.generalize(addr, slots)
    }
    async _fetchImageFromDataUri(dataUri: string): Promise<string | undefined> {
        try {
            const resp = await fetch(dataUri)
            const data = await resp.json()
            return data.image
        } catch (e) {
            return undefined
        }
    }
    async _getPrioritizationFee(pubkeys: PublicKey[]): Promise<number> {
        const prioritizationFee = await this.#conn.getRecentPrioritizationFees({
            lockedWritableAccounts: pubkeys,
        })
        let f = 10000
        for (const fee of prioritizationFee) {
            f += fee.prioritizationFee
        }
        console.info('Estimated prioritization fee:', f)
        return f
    }
    async getTokenDecimals(addr: string): Promise<number | undefined> {
        const mintKey = new PublicKey(addr)
        const mint = await getMint(this.#conn, mintKey, 'finalized')
        return mint.decimals
    }
    async getToken(addr: string): Promise<Token | undefined> {
        const mintKey = new PublicKey(addr)
        try {
            const mint = await getMint(this.#conn, mintKey, 'finalized')
            const [metadataKey] = PublicKey.findProgramAddressSync(
                [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
                METAPLEX_PROGRAM_ID
            )
            const metadataAccount = await this.#conn.getAccountInfo(metadataKey)
            let metadataState
            if (metadataAccount !== null) {
                metadataState = Metadata.deserialize(metadataAccount.data)
            }
            // Get from legacy token
            if (this.#legacyTokens === undefined) {
                await this._fetchLegacyTokens()
            }
            let legacyToken: LegacyToken | undefined = undefined
            if (this.#legacyTokens !== undefined) {
                legacyToken = this.#legacyTokens.find(t => t.address === addr)
            }
            if (metadataState !== undefined) {
                const addr = mint.address.toBase58()
                const decimals = mint.decimals
                const image = await this._fetchImageFromDataUri(metadataState.data.uri)
                const name = metadataState.data.name
                    ? trimString(metadataState.data.name)
                    : legacyToken
                      ? legacyToken.name
                      : ''
                const symbol = metadataState.data.symbol
                    ? trimString(metadataState.data.symbol)
                    : legacyToken
                      ? legacyToken.symbol
                      : ''
                const icon = image ? image : legacyToken?.logoURI ? legacyToken.logoURI : ''
                return { addr, decimals, name, symbol, icon }
            } else {
                return undefined
            }
        } catch (e) {
            console.warn(e)
            return undefined
        }
    }
    // Return [name, symbol, icon]
    async parseTokenMetadata(
        addr: string,
        metadataAccount: AccountInfo<Buffer>
    ): Promise<[string | undefined, string | undefined, string | undefined]> {
        const metadataState = Metadata.deserialize(metadataAccount.data)
        const uri = trimString(metadataState.data.uri)
        const image = uri ? await this._fetchImageFromDataUri(uri) : undefined
        if (this.#legacyTokens === undefined) {
            await this._fetchLegacyTokens()
        }
        let legacyToken: LegacyToken | undefined = undefined
        if (this.#legacyTokens !== undefined) {
            legacyToken = this.#legacyTokens.find(t => t.address === addr)
        }
        const name = metadataState.data.name ? trimString(metadataState.data.name) : legacyToken ? legacyToken.name : ''
        const symbol = metadataState.data.symbol
            ? trimString(metadataState.data.symbol)
            : legacyToken
              ? legacyToken.symbol
              : ''
        const icon = image ? image : legacyToken?.logoURI ? legacyToken.logoURI : ''
        return [name, symbol, icon]
    }
    async listTokens(tokenAddrs: string[]): Promise<Token[]> {
        if (tokenAddrs.length > 30) {
            throw new Error('Too many token addresses in a row')
        }
        let results = []
        let mintMetaList: [PublicKey, PublicKey][] = []
        for (const t of tokenAddrs) {
            const mintKey = new PublicKey(t)
            if (mintKey.equals(NATIVE_MINT)) {
                results.push(SOL_TOKEN)
            } else {
                const [metadataKey] = PublicKey.findProgramAddressSync(
                    [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
                    METAPLEX_PROGRAM_ID
                )
                mintMetaList.push([mintKey, metadataKey])
            }
        }
        const accountInfos = await this.#conn.getMultipleAccountsInfo(mintMetaList.flat())
        for (let i = 0; i < mintMetaList.length; i++) {
            const mintKey = mintMetaList[i][0]
            const mintAccount = accountInfos[2 * i]
            const metadataAccount = accountInfos[2 * i + 1]
            let addr = mintKey.toBase58()
            let decimals: number | undefined = undefined
            let name: string | undefined = undefined
            let symbol: string | undefined = undefined
            let icon: string | undefined = undefined
            if (mintAccount) {
                const m = MintLayout.decode(mintAccount.data)
                decimals = m.decimals
            }
            if (metadataAccount) {
                ;[name, symbol, icon] = await this.parseTokenMetadata(addr, metadataAccount)
            }
            if (decimals !== undefined && name !== undefined && symbol !== undefined && icon !== undefined) {
                const token = { addr, name, symbol, icon, decimals }
                console.debug('Found token:', token)
                results.push(token)
            } else if (decimals !== undefined) {
                const token = { addr, name: '', symbol: '', icon: '', decimals }
                console.debug('Found token(missing metadata):', token)
                results.push(token)
            }
        }
        return results
    }
    /**
     * List tokens.
     */
    async listTokensWithBalance(walletAddr: string, tokenAddrs: string[]): Promise<TokenWithBalance[]> {
        if (tokenAddrs.length > 30) {
            throw new Error('Too many token addresses in a row')
        }
        let results = []
        const ownerKey = new PublicKey(walletAddr)
        let mintAtaList: [PublicKey, PublicKey, PublicKey][] = []
        for (const t of tokenAddrs) {
            const mintKey = new PublicKey(t)
            if (mintKey.equals(NATIVE_MINT)) {
                const lamports = await this.#conn.getBalance(ownerKey)
                results.push(new TokenWithBalance(SOL_TOKEN, BigInt(lamports)))
            } else {
                const ataKey = getAssociatedTokenAddressSync(mintKey, ownerKey)
                const [metadataKey] = PublicKey.findProgramAddressSync(
                    [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
                    METAPLEX_PROGRAM_ID
                )
                mintAtaList.push([mintKey, ataKey, metadataKey])
            }
        }
        const accountInfos = await this.#conn.getMultipleAccountsInfo(mintAtaList.flat())
        for (let i = 0; i < mintAtaList.length; i++) {
            const mintKey = mintAtaList[i][0]
            const mintAccount = accountInfos[3 * i]
            const ataAccount = accountInfos[3 * i + 1]
            const metadataAccount = accountInfos[3 * i + 2]
            let addr = mintKey.toBase58()
            let decimals: number | undefined = undefined
            let name: string | undefined = undefined
            let symbol: string | undefined = undefined
            let icon: string | undefined = undefined
            let balance: bigint = 0n
            if (mintAccount) {
                const m = MintLayout.decode(mintAccount.data)
                decimals = m.decimals
            }
            if (metadataAccount) {
                ;[name, symbol, icon] = await this.parseTokenMetadata(addr, metadataAccount)
            }
            if (ataAccount) {
                const acc = AccountLayout.decode(ataAccount.data)
                balance = acc.amount
            }
            if (decimals !== undefined && name !== undefined && symbol !== undefined && icon !== undefined) {
                const tokenWithBalance = new TokenWithBalance({ addr, name, symbol, icon, decimals }, balance)
                console.debug('Found token with balance:', tokenWithBalance)
                results.push(tokenWithBalance)
            } else if (decimals !== undefined) {
                const tokenWithBalance = new TokenWithBalance(
                    { addr, name: '', symbol: '', icon: '', decimals },
                    balance
                )
                console.debug('Found token with balance(missing metadata):', tokenWithBalance)
                results.push(tokenWithBalance)
            }
        }
        return results
    }
    async getNft(addr: string | PublicKey): Promise<Nft | undefined> {
        let mintKey: PublicKey
        if (addr instanceof PublicKey) {
            mintKey = addr
        } else {
            mintKey = new PublicKey(addr)
        }
        try {
            const mint = await getMint(this.#conn, mintKey, 'finalized')
            // Non-zero decimals stands for a fungible token
            if (mint.decimals !== 0) {
                return undefined
            }
            const [metadataKey] = PublicKey.findProgramAddressSync(
                [Buffer.from('metadata', 'utf8'), METAPLEX_PROGRAM_ID.toBuffer(), mintKey.toBuffer()],
                METAPLEX_PROGRAM_ID
            )
            const metadataAccount = await this.#conn.getAccountInfo(metadataKey)
            let metadataState
            if (metadataAccount !== null) {
                metadataState = Metadata.deserialize(metadataAccount.data)
            }
            if (metadataState !== undefined) {
                const image = await this._fetchImageFromDataUri(metadataState.data.uri)
                if (image === undefined) return undefined
                const nft = {
                    addr: mint.address.toBase58(),
                    name: trimString(metadataState.data.name),
                    symbol: trimString(metadataState.data.symbol),
                    image,
                    collection: metadataState?.collection?.key.toBase58(),
                    metadata: metadataState?.data,
                }
                return nft
            } else {
                return undefined
            }
        } catch (e) {
            console.warn(e)
            return undefined
        }
    }
    async listNfts(walletAddr: string): Promise<Nft[]> {
        let nfts = []
        const ownerKey = new PublicKey(walletAddr)
        const parsedTokenAccounts = await this.#conn.getParsedTokenAccountsByOwner(ownerKey, {
            programId: TOKEN_PROGRAM_ID,
        })
        for (const a of parsedTokenAccounts.value) {
            if (
                a.account.data.parsed.info.tokenAmount.amount !== '1' ||
                a.account.data.parsed.info.tokenAmount.decimals !== 0
            ) {
                continue
            }
            const nft = await this.getNft(a.account.data.parsed.info.mint)
            if (nft !== undefined) {
                nfts.push(nft)
            }
        }
        return nfts
    }
    async _getGameState(gameAccountKey: PublicKey): Promise<GameState | undefined> {
        const conn = this.#conn
        const gameAccount = await conn.getAccountInfo(gameAccountKey)
        if (gameAccount !== null) {
            const data = gameAccount.data
            return GameState.deserialize(data)
        } else {
            return undefined
        }
    }
    async _getMultiGameStates(gameAccountKeys: PublicKey[]): Promise<Array<GameState | undefined>> {
        const conn = this.#conn
        const accountsInfo = await conn.getMultipleAccountsInfo(gameAccountKeys)
        const ret: Array<GameState | undefined> = []
        console.info('Get %s games from registry', accountsInfo.length)
        for (let i = 0; i < accountsInfo.length; i++) {
            const key = gameAccountKeys[i]
            const accountInfo = accountsInfo[i]
            if (accountInfo !== null) {
                try {
                    ret.push(GameState.deserialize(accountInfo.data))
                    console.info('Found game account %s', key)
                } catch (_: any) {
                    ret.push(undefined)
                    console.warn('Skip invalid game account %s', key)
                }
            } else {
                ret.push(undefined)
                console.warn('Game account %s not exist', key)
            }
        }
        return ret
    }
    async _getRecipientState(recipientKey: PublicKey): Promise<RecipientState | undefined> {
        const conn = this.#conn
        const recipientAccount = await conn.getAccountInfo(recipientKey)
        if (recipientAccount !== null) {
            const data = recipientAccount.data
            return RecipientState.deserialize(data)
        } else {
            return undefined
        }
    }
    async _getRegState(regKey: PublicKey): Promise<RegistryState | undefined> {
        const conn = this.#conn
        const regAccount = await conn.getAccountInfo(regKey)
        if (regAccount !== null) {
            const data = regAccount.data
            return RegistryState.deserialize(data)
        } else {
            return undefined
        }
    }
    async _getServerState(serverKey: PublicKey): Promise<ServerState | undefined> {
        const conn = this.#conn
        const profileKey = await PublicKey.createWithSeed(serverKey, SERVER_PROFILE_SEED, PROGRAM_ID)
        const serverAccount = await conn.getAccountInfo(profileKey)
        if (serverAccount !== null) {
            const data = serverAccount.data
            return ServerState.deserialize(data)
        } else {
            return undefined
        }
    }
}
async function sendTransaction<T, E>(
    wallet: IWallet,
    tx: VersionedTransaction,
    conn: Connection,
    response: ResponseHandle<T, E>,
    config?: SendTransactionOptions
): Promise<SendTransactionResult> {
    const w = (wallet as SolanaWalletAdapter).wallet
    const {
        value: { blockhash, lastValidBlockHeight },
    } = await conn.getLatestBlockhashAndContext()
    console.info('Signers: ', config?.signers)
    response.waitingWallet()
    if (config?.signers !== undefined) {
        tx.sign(config.signers)
    }
    let signedTransaction
    try {
        signedTransaction = await w.signTransaction(tx)
    } catch (e: any) {
        console.error(e)
        response.userRejected(e.toString())
        return { err: e }
    }
    console.info('Signed transaction:', signedTransaction)
    try {
        let simulationResult = await conn.simulateTransaction(signedTransaction, { sigVerify: true })
        console.debug('Transaction simulation result:', simulationResult)
        if (simulationResult.value.err) {
            if (simulationResult.value.logs !== null) {
                for (const log of simulationResult.value.logs) {
                    console.warn(log)
                }
            }
            console.error(simulationResult.value.err)
            response.transactionFailed('simulation error')
            return { err: 'simulation-error' }
        }
    } catch (e: any) {
        console.error(e)
        response.userRejected(e.toString())
        return { err: e.toString() }
    }
    let signature: TransactionSignature
    try {
        signature = await conn.sendRawTransaction(signedTransaction.serialize())
        response.confirming(signature)
    } catch (e: any) {
        console.error(e)
        response.userRejected(e.toString())
        return { err: e.toString() }
    }
    try {
        const resp = await conn.confirmTransaction({ blockhash, lastValidBlockHeight, signature }, config?.commitment)
        if (resp.value.err !== null) {
            return { err: { signature, error: resp.value.err } }
        } else {
            return { ok: signature }
        }
    } catch (e: any) {
        return { err: e.toString() }
    }
}
async function makeTransaction(
    conn: Connection,
    feePayer: PublicKey,
    instructions: TransactionInstruction[]
): Promise<Result<VersionedTransaction, string>> {
    const d = new Date()
    const slot = await conn.getSlot()
    let block
    try {
        block = await conn.getBlock(slot, { maxSupportedTransactionVersion: 0, transactionDetails: 'none' })
    } catch (e: any) {
        return { err: 'block-not-found' }
    }
    if (block === null) {
        return { err: 'block-not-found' }
    }
    console.debug('Got block hash %s, took %s milliseconds', block.blockhash, new Date().getTime() - d.getTime())
    const messageV0 = new TransactionMessage({
        payerKey: feePayer,
        recentBlockhash: block.blockhash,
        instructions,
    }).compileToV0Message()
    const transaction = new VersionedTransaction(messageV0)
    return { ok: transaction }
}
