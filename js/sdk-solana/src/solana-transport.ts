import {
    Commitment,
    Rpc,
    createSolanaRpcFromTransport,
    createDefaultRpcTransport,
    SolanaRpcApi,
    createTransactionMessage,
    setTransactionMessageFeePayer,
    setTransactionMessageLifetimeUsingBlockhash,
    address,
    Address,
    TransactionMessage,
    pipe,
    partiallySignTransaction,
    appendTransactionMessageInstructions,
    IInstruction,
    generateKeyPairSigner,
    getProgramDerivedAddress,
    KeyPairSigner,
    TransactionSigner,
    TransactionSendingSigner,
    compileTransaction,
    TransactionMessageWithBlockhashLifetime,
    ITransactionMessageWithFeePayer,
    createAddressWithSeed,
    Blockhash,
    Transaction,
    Signature,
    getBase58Encoder,
    getBase58Decoder,
} from '@solana/web3.js'
import * as SPL from '@solana-program/token'
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
    TokenBalance,
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
    RECIPIENT_ACCOUNT_LEN,
    NATIVE_MINT,
    SERVER_PROFILE_SEED,
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
import { getCreateAccountInstruction, getCreateAccountWithSeedInstruction } from '@solana-program/system'
import { TOKEN_PROGRAM_ADDRESS } from '@solana-program/token'
import { createDasRpc, MetaplexDASApi } from './metaplex'

const MAX_CONFIRM_TIMES = 32

type TransactionMessageWithFeePayerAndBlockhashLifetime = TransactionMessage &
    ITransactionMessageWithFeePayer &
    TransactionMessageWithBlockhashLifetime

function base64ToUint8Array(base64: string): Uint8Array {
    const rawBytes = atob(base64)
    const uint8Array = new Uint8Array(rawBytes.length)
    for (let i = 0; i < rawBytes.length; i++) {
        uint8Array[i] = rawBytes.charCodeAt(i)
    }
    return uint8Array
}

function trimString(s: string): string {
    return s.replace(/\0/g, '')
}

function getSigner(wallet: IWallet): TransactionSendingSigner {
    return (wallet as SolanaWalletAdapter).wallet
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
    signers?: KeyPairSigner[]
    commitment?: Commitment
}

export class SolanaTransport implements ITransport {
    #rpc: Rpc<SolanaRpcApi>
    #dasRpc: Rpc<MetaplexDASApi>
    #endpoint: string

    constructor(endpoint: string) {
        const transport = createDefaultRpcTransport({ url: endpoint })
        this.#endpoint = endpoint
        this.#rpc = createSolanaRpcFromTransport(transport)
        this.#dasRpc = createDasRpc(transport)
    }

    get chain(): Chain {
        return 'solana'
    }

    async createGameAccount(
        wallet: IWallet,
        params: CreateGameAccountParams,
        response: ResponseHandle<CreateGameResponse, CreateGameError>
    ): Promise<void> {
        const { title, bundleAddr, tokenAddr } = params
        if (title.length > NAME_LEN) {
            return response.failed('invalid-title')
        }

        const payer = getSigner(wallet)

        const recipientAccountKey = address(params.recipientAddr)

        const registrationAccountKey = address(params.registrationAddr)

        let ixs: IInstruction[] = []
        let signers: KeyPairSigner[] = []

        const { ixs: createGameAccountIxs, account: gameAccount } = await this._prepareCreateAccount(
            payer,
            GAME_ACCOUNT_LEN,
            PROGRAM_ID
        )
        ixs.push(...createGameAccountIxs)
        signers.push(gameAccount)

        const tokenMintKey = address(tokenAddr)

        let stakeAccountKey
        if (tokenMintKey == NATIVE_MINT) {
            // For SOL game, use PDA as stake account
            const [pda, _] = await getProgramDerivedAddress({
                programAddress: PROGRAM_ID,
                seeds: [getBase58Encoder().encode(gameAccount.address)],
            })
            stakeAccountKey = pda
            console.info('Game uses SOL as token, use PDA as stake account:', stakeAccountKey)
        } else {
            // For SPL game, use dedicated stake account
            const { tokenAccount: stakeAccount, ixs: createStakeAccountIxs } = await this._prepareCreateTokenAccount(
                payer,
                tokenMintKey
            )
            signers.push(stakeAccount)
            ixs.push(...createStakeAccountIxs)
            stakeAccountKey = stakeAccount.address
            console.info('Game uses SPL as token, use dedicated stake account:', stakeAccountKey)
        }

        const bundleKey = address(bundleAddr)
        const createGame = instruction.createGameAccount({
            ownerKey: payer.address,
            gameAccountKey: gameAccount.address,
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
            ownerKey: payer.address,
            gameAccountKey: gameAccount.address,
            registrationAccountKey,
        })

        console.info('Transaction Instruction[RegisterGame]:', registerGame)
        ixs.push(registerGame)

        const tx = await makeTransaction(this.#rpc, payer, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }

        const sig = await sendTransaction(payer, tx.ok, response, { signers })
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { gameAddr: gameAccount.address, signature })
    }

    async closeGameAccount(
        wallet: IWallet,
        params: CloseGameAccountParams,
        response: ResponseHandle<CloseGameAccountResponse, CloseGameAccountError>
    ): Promise<void> {
        const { gameAddr, regAddr } = params

        const payer = getSigner(wallet)
        const gameAccountKey = address(gameAddr)
        const regAccountKey = address(regAddr)

        const gameState = await this._getGameState(gameAccountKey)

        if (gameState === undefined) {
            return response.failed('game-not-found')
        }

        if (gameState.ownerKey != payer.address) {
            return response.failed('permission-denied')
        }
        const regState = await this._getRegState(regAccountKey)

        if (regState === undefined) {
            return response.failed('reg-not-found')
        }

        if (regState.games.find(g => g.gameKey == gameAccountKey) === undefined) {
            return response.failed('game-not-in-reg')
        }
        const ixs = []
        const [pda, _] = await getProgramDerivedAddress({ programAddress: PROGRAM_ID, seeds: [getBase58Encoder().encode(gameAccountKey)] })

        let receiver

        if (gameState.tokenKey == NATIVE_MINT) {
            receiver = payer.address
        } else {
            ;[receiver] = await SPL.findAssociatedTokenPda({
                owner: payer.address,
                tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS,
                mint: gameState.tokenKey,
            })
        }

        const unregisterGameIx = instruction.unregisterGame({
            payerKey: payer.address,
            regAccountKey,
            gameAccountKey,
        })
        ixs.push(unregisterGameIx)
        const stakeKey = gameState.stakeKey
        const closeGameAccountIx = await instruction.closeGame({
            payerKey: payer.address,
            gameAccountKey,
            stakeKey,
            pda,
            receiver,
            gameState,
        })
        ixs.push(closeGameAccountIx)
        const tx = await makeTransaction(this.#rpc, payer, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }
        const sig = await sendTransaction(payer, tx.ok, response, { commitment: 'confirmed' })

        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { signature })
    }

    async join(wallet: IWallet, params: JoinParams, response: ResponseHandle<JoinResponse, JoinError>): Promise<void> {
        const { gameAddr, amount: amountRaw, position, verifyKey } = params
        const gameAccountKey = address(gameAddr)
        const player = getSigner(wallet)

        // Call RPC functions in Parallel
        const d = new Date()
        const [gameState, playerProfile] = await Promise.all([
            this._getGameState(gameAccountKey),
            this.getPlayerProfile(wallet.walletAddr),
        ])
        console.debug('Batched RPC calls took %s milliseconds', new Date().getTime() - d.getTime())

        const profileKey0 = playerProfile !== undefined ? address(playerProfile?.addr) : undefined

        if (gameState === undefined) return response.failed('game-not-found')

        const accessVersion = gameState.accessVersion
        const settleVersion = gameState.settleVersion

        const mintKey = gameState.tokenKey
        const isWsol = mintKey == NATIVE_MINT
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

        let ixs: IInstruction[] = []

        let profileKey: Address
        if (profileKey0 !== undefined) {
            profileKey = profileKey0
        } else if (params.createProfileIfNeeded) {
            const createProfile = await this._prepareCreatePlayerProfile(player, {
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
            const account = await generateKeyPairSigner()
            const ix = getCreateAccountInstruction({
                payer: player,
                newAccount: account,
                lamports: amount,
                space: 0,
                programAddress: PROGRAM_ID,
            })
            ixs.push(ix)
            tempAccount = account
        } else {
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                player,
                mintKey
            )
            ixs.push(...createTempAccountIxs)

            const [playerAta] = await SPL.findAssociatedTokenPda({
                owner: player.address,
                mint: mintKey,
                tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS,
            })
            const transferIx = SPL.getTransferInstruction({
                amount,
                authority: player,
                source: playerAta,
                destination: tokenAccount.address,
            })
            ixs.push(transferIx)
            tempAccount = tokenAccount
        }

        let [pda] = await getProgramDerivedAddress({ programAddress: PROGRAM_ID, seeds: [getBase58Encoder().encode(gameAccountKey)] })

        const joinGameIx = instruction.join({
            playerKey: player.address,
            profileKey,
            paymentKey: tempAccount.address,
            gameAccountKey,
            mint: mintKey,
            stakeAccountKey,
            recipientAccountKey: gameState.recipientAddr,
            amount,
            accessVersion,
            settleVersion,
            position,
            verifyKey,
            pda,
        })

        console.info('Transaction Instruction[Join]:', joinGameIx)
        ixs.push(joinGameIx)

        const tx = await makeTransaction(this.#rpc, player, ixs)
        if ('err' in tx) {
            response.retryRequired(tx.err)
            return
        }

        const sig = await sendTransaction(player, tx.ok, response, {
            commitment: 'confirmed',
            signers: [tempAccount],
        })
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { signature })
    }

    async deposit(
        wallet: IWallet,
        params: DepositParams,
        response: ResponseHandle<DepositResponse, DepositError>
    ): Promise<void> {
        const player = getSigner(wallet)
        const gameAccountKey = address(params.gameAddr)
        // Call RPC functions in Parallel
        const [gameState, playerProfile] = await Promise.all([
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
            profileKey = address(playerProfile.addr)
        }
        if (gameState.transactorKey === undefined) {
            return response.failed('game-not-served')
        }
        const settleVersion = gameState.settleVersion
        const mintKey = gameState.tokenKey
        const isWsol = mintKey == NATIVE_MINT
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

        let tempAccount
        if (isWsol) {
            const account = await generateKeyPairSigner()

            const ix = getCreateAccountInstruction({
                payer: player,
                newAccount: account,
                lamports: amount,
                space: 0,
                programAddress: PROGRAM_ID,
            })
            ixs.push(ix)
            tempAccount = account
        } else {
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                player,
                mintKey
            )
            ixs.push(...createTempAccountIxs)

            const [playerAta] = await SPL.findAssociatedTokenPda({
                owner: player.address,
                mint: mintKey,
                tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS,
            })
            const transferIx = SPL.getTransferInstruction({
                amount,
                authority: player,
                source: playerAta,
                destination: tokenAccount.address,
            })

            ixs.push(transferIx)
            tempAccount = tokenAccount
        }

        const [pda, _] = await getProgramDerivedAddress({ programAddress: PROGRAM_ID, seeds: [getBase58Encoder().encode(gameAccountKey)] })

        const depositGameIx = instruction.deposit({
            playerKey: player.address,
            profileKey,
            paymentKey: tempAccount.address,
            gameAccountKey,
            mint: mintKey,
            stakeAccountKey: gameState.stakeKey,
            recipientAccountKey: gameState.recipientAddr,
            amount,
            settleVersion,
            pda,
        })
        console.info('Transaction Instruction[Deposit]:', depositGameIx)
        ixs.push(depositGameIx)
        const tx = await makeTransaction(this.#rpc, player, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(player, tx.ok, response, {
            commitment: 'confirmed',
            signers: [tempAccount],
        })
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { signature })
    }

    async attachBonus(
        wallet: IWallet,
        params: AttachBonusParams,
        response: ResponseHandle<AttachBonusResponse, AttachBonusError>
    ): Promise<void> {
        const payer = getSigner(wallet)
        const gameAccountKey = address(params.gameAddr)
        const gameState = await this._getGameState(gameAccountKey)
        if (gameState === undefined) {
            return response.failed('game-not-found')
        }
        let ixs = []
        let tempAccountKeys = []
        let signers = []

        for (const bonus of params.bonuses) {
            const { tokenAddr, amount } = bonus
            const mintKey = address(tokenAddr)
            const mint = await SPL.fetchMint(this.#rpc, mintKey, { commitment: 'finalized' })
            const { ixs: createTempAccountIxs, tokenAccount: tokenAccount } = await this._prepareCreateTokenAccount(
                payer,
                mintKey
            )
            ixs.push(...createTempAccountIxs)
            const [playerAta] = await SPL.findAssociatedTokenPda({
                owner: payer.address,
                tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS,
                mint: mintKey,
            })

            const transferIx = SPL.getTransferCheckedInstruction({
                source: playerAta,
                mint: mintKey,
                destination: tokenAccount.address,
                amount,
                decimals: mint.data.decimals,
                authority: payer.address,
            })

            ixs.push(transferIx)
            tempAccountKeys.push(tokenAccount.address)
            signers.push(tokenAccount)
        }

        const attachBonusIx = instruction.attachBonus({
            payerKey: payer.address,
            gameAccountKey: address(params.gameAddr),
            stakeAccountKey: gameState.stakeKey,
            identifiers: params.bonuses.map(b => b.identifier),
            tempAccountKeys,
        })

        if ('err' in attachBonusIx) {
            return response.failed(attachBonusIx.err)
        }
        console.info('Transaction Instruction[attachBonus]:', attachBonusIx.ok)
        ixs.push(attachBonusIx.ok)
        const tx = await makeTransaction(this.#rpc, payer, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(payer, tx.ok, response, { signers })
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { signature })
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
        const payer = getSigner(wallet)
        const recipientKey = address(params.recipientAddr)
        const recipientState = await this._getRecipientState(recipientKey)
        if (recipientState === undefined) {
            return response.failed('not-found')
        }

        const recipientClaimIx = await instruction.claim({
            recipientKey,
            payerKey: payer.address,
            recipientState,
        })
        if ('err' in recipientClaimIx) {
            return response.failed(recipientClaimIx.err)
        }
        const tx = await makeTransaction(this.#rpc, payer, [recipientClaimIx.ok])
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(payer, tx.ok, response)
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { recipientAddr: params.recipientAddr, signature })
    }

    async _getPlayerProfileAddress(payerKey: Address) {
        return await createAddressWithSeed({
            baseAddress: payerKey,
            programAddress: PROGRAM_ID,
            seed: PLAYER_PROFILE_SEED,
        })
    }

    async _getServerProfileAddress(serverKey: Address) {
        return await createAddressWithSeed({
            baseAddress: serverKey,
            programAddress: PROGRAM_ID,
            seed: SERVER_PROFILE_SEED,
        })
    }

    async _prepareCreatePlayerProfile(
        payer: TransactionSendingSigner,
        params: CreatePlayerProfileParams
    ): Promise<Result<{ ixs: IInstruction[]; profileKey: Address }, CreatePlayerProfileError>> {
        let ixs = []
        const { nick, pfp } = params
        if (nick.length > 16) {
            return { err: 'invalid-nick' }
        }
        console.info('Payer Public Key:', payer.address)

        const profileKey = await this._getPlayerProfileAddress(payer.address)

        console.info('Player profile public key: ', profileKey)
        const profileAccountData = await this._getFinializedBase64AccountData(profileKey)

        if (!profileAccountData) {
            const lamports = await this.#rpc.getMinimumBalanceForRentExemption(PROFILE_ACCOUNT_LEN).send()
            const ix = getCreateAccountWithSeedInstruction({
                baseAccount: payer,
                payer: payer,
                newAccount: profileKey,
                space: PROFILE_ACCOUNT_LEN,
                programAddress: PROGRAM_ID,
                seed: PLAYER_PROFILE_SEED,
                amount: lamports,
                base: payer.address,
            })
            console.info('Transaction Instruction[CreateAccount]:', ix)
            ixs.push(ix)
        }

        const pfpKey = !pfp ? address('11111111111111111111111111111111') : address(pfp)
        const createProfile = instruction.createPlayerProfile(payer.address, profileKey, nick, pfpKey)
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
        let ixs: IInstruction[] = []
        const payer = getSigner(wallet)
        const createPlayerProfile = await this._prepareCreatePlayerProfile(payer, params)
        if ('err' in createPlayerProfile) {
            return response.failed(createPlayerProfile.err)
        }
        const { ixs: createProfileIxs, profileKey } = createPlayerProfile.ok
        ixs.push(...createProfileIxs)
        let tx = await makeTransaction(this.#rpc, payer, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const sig = await sendTransaction(payer, tx.ok, response)
        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, {
            signature,
            profile: {
                nick: params.nick,
                pfp: params.pfp,
                addr: profileKey,
            },
        })
    }

    async _prepareCreateTokenAccount(
        payer: TransactionSigner,
        mint: Address
    ): Promise<{ ixs: IInstruction[]; tokenAccount: KeyPairSigner }> {
        const token = await generateKeyPairSigner()
        const space = SPL.getTokenSize()
        const rent = await this.#rpc.getMinimumBalanceForRentExemption(BigInt(space)).send()

        const ixs = [
            getCreateAccountInstruction({
                payer,
                newAccount: token,
                lamports: rent,
                space,
                programAddress: TOKEN_PROGRAM_ADDRESS,
            }),
            SPL.getInitializeAccountInstruction({
                account: token.address,
                mint,
                owner: payer.address,
            }),
        ]

        return {
            ixs,
            tokenAccount: token,
        }
    }
    async _prepareCreateAccount(
        payer: TransactionSigner,
        size: bigint,
        programAddress: Address
    ): Promise<{ ixs: IInstruction[]; account: KeyPairSigner }> {
        const account = await generateKeyPairSigner()
        const lamports = await this.#rpc.getMinimumBalanceForRentExemption(size).send()

        const ix = getCreateAccountInstruction({
            payer,
            newAccount: account,
            space: size,
            lamports,
            programAddress,
        })

        console.info('Transaction Instruction[CreateAccount]:', ix)
        return { ixs: [ix], account }
    }
    async _prepareCreateRecipient(
        payer: TransactionSigner,
        params: CreateRecipientParams
    ): Promise<
        Result<{ recipientAccount: KeyPairSigner; ixs: IInstruction[]; signers: KeyPairSigner[] }, CreateRecipientError>
    > {
        if (params.slots.length > 10) {
            return { err: 'invalid-size' }
        }
        let ixs: IInstruction[] = []
        let signers: KeyPairSigner[] = []
        let capKey: Address
        if (params.capAddr === undefined) {
            capKey = payer.address
        } else {
            capKey = address(params.capAddr)
        }
        // Create Recipient Account
        let { ixs: createRecipientAccountIxs, account: recipientAccount } = await this._prepareCreateAccount(
            payer,
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
            let stakeAddr: Address
            if (slot.tokenAddr === NATIVE_MINT) {
                // Use PDA as stake account for SOL slot
                const [pda] = await getProgramDerivedAddress({
                    programAddress: PROGRAM_ID,
                    seeds: [recipientAccount.address, Uint8Array.of(slot.id)],
                })

                stakeAddr = pda
            } else {
                // Use dedicated stake account
                const { ixs: createStakeAccountIxs, tokenAccount: stakeAccount } =
                    await this._prepareCreateTokenAccount(payer, address(slot.tokenAddr))
                ixs.push(...createStakeAccountIxs)
                signers.push(stakeAccount)
                stakeAddr = stakeAccount.address
            }
            const slotInit = new instruction.SlotInit({
                id: slot.id,
                tokenAddr: address(slot.tokenAddr),
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
            payerKey: payer.address,
            recipientKey: recipientAccount.address,
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
        const payer = getSigner(wallet)
        const createRecipient = await this._prepareCreateRecipient(payer, params)
        if ('err' in createRecipient) {
            return response.failed(createRecipient.err)
        }
        const { ixs, recipientAccount, signers } = createRecipient.ok
        const tx = await makeTransaction(this.#rpc, payer, ixs)
        if ('err' in tx) {
            return response.retryRequired(tx.err)
        }
        const transaction = tx.ok
        const sig = await sendTransaction(payer, transaction, response, { signers })

        if ('err' in sig) {
            return response.transactionFailed(sig.err)
        }

        const signature = sig.ok

        await confirmSignature(this.#rpc, signature, response, { recipientAddr: recipientAccount.address, signature })
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
        const gameAccountKey = address(addr)
        const gameState = await this._getGameState(gameAccountKey)
        if (gameState !== undefined) {
            return gameState.generalize(address(addr))
        } else {
            return undefined
        }
    }
    async getGameBundle(addr: string): Promise<GameBundle | undefined> {
        const mintKey = address(addr)
        const [metadataKey] = await getProgramDerivedAddress({
            programAddress: METAPLEX_PROGRAM_ID,
            seeds: ['metadata', getBase58Encoder().encode(METAPLEX_PROGRAM_ID), getBase58Encoder().encode(mintKey)],
        })

        const metadataAccountData = await this._getFinializedBase64AccountData(metadataKey)
        if (metadataAccountData === undefined) {
            return undefined
        }
        const metadataState = Metadata.deserialize(metadataAccountData)
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
        const playerKey = address(addr)

        const profileKey = await this._getPlayerProfileAddress(playerKey)

        const profileAccountData = await this._getFinializedBase64AccountData(profileKey)

        if (profileAccountData !== undefined) {
            const state = PlayerState.deserialize(profileAccountData)
            return state.generalize(playerKey)
        } else {
            return undefined
        }
    }
    async listPlayerProfiles(addrs: string[]): Promise<Array<PlayerProfile | undefined>> {
        // We should truncate addresses by 100
        let results: Array<PlayerProfile | undefined> = []
        for (let i = 0; i < addrs.length; i += 100) {
            const addrsChunk = addrs.slice(i, i + 100).map(address)
            const keys = await Promise.all(addrsChunk.map(addr => this._getPlayerProfileAddress(addr)))
            const states = await this._getMultiPlayerStates(keys)
            results.push(...states.map((state, j) => state?.generalize(addrsChunk[j])))
        }
        return results
    }
    async getServerAccount(addr: string): Promise<ServerAccount | undefined> {
        const serverKey = address(addr)

        const profileKey = await this._getServerProfileAddress(serverKey)
        const serverState = await this._getServerState(profileKey)
        if (serverState !== undefined) {
            return serverState.generalize()
        } else {
            return undefined
        }
    }
    async getRegistration(addr: string): Promise<RegistrationAccount | undefined> {
        const regKey = address(addr)
        const regState = await this._getRegState(regKey)
        if (regState !== undefined) {
            return regState.generalize(regKey)
        } else {
            return undefined
        }
    }

    async listGameAccounts(addrs: string[]): Promise<GameAccount[]> {
        const keys = addrs.map(a => address(a))
        const gameStates = await this._getMultiGameStates(keys)
        let games: Array<GameAccount> = []
        for (let i = 0; i < gameStates.length; i++) {
            const gs = gameStates[i]
            if (gs !== undefined) {
                games.push(gs.generalize(keys[i]))
            }
        }
        return games
    }

    async getRecipient(addr: string): Promise<RecipientAccount | undefined> {
        const recipientKey = address(addr)
        const recipientState = await this._getRecipientState(recipientKey)
        if (recipientState === undefined) return undefined
        let slots: RecipientSlot[] = []
        for (const slot of recipientState.slots) {
            let balance
            if (slot.tokenAddr == NATIVE_MINT) {
                const resp = (await this.#rpc.getAccountInfo(slot.stakeAddr).send()).value
                balance = BigInt(resp?.lamports || 0n)
            } else {
                const resp = await this.#rpc.getTokenAccountBalance(slot.stakeAddr).send()
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

    async getTokenDecimals(addr: string): Promise<number | undefined> {
        const mintKey = address(addr)

        const mint = await SPL.fetchMint(this.#rpc, mintKey, { commitment: 'finalized' })

        return mint.data.decimals
    }

    async _getAssetAsToken(addr: Address): Promise<Token | undefined> {
        const assetResp = await this.#dasRpc.getAsset(addr).send()
        if ('result' in assetResp) {
            const asset = assetResp.result
            const { name, symbol } = asset.content.metadata
            const icon = asset.content.files?.[0]?.uri
            if (icon == undefined) {
                console.warn('Skip token %s as its icon is not available', addr)
                return undefined
            }
            const decimals = asset.token_info.decimals
            const token = {
                addr,
                name,
                symbol,
                icon,
                decimals,
            }
            return token
        } else {
            console.warn(assetResp.error, 'Error in getAsset response')
            return undefined
        }
    }

    async getToken(addr: string): Promise<Token | undefined> {
        const mintKey = address(addr)
        try {
            return await this._getAssetAsToken(mintKey)
        } catch (e) {
            console.warn(e)
            return undefined
        }
    }

    async listTokens(rawMintAddrs: string[]): Promise<Token[]> {
        // In Solana, token specification is stored in Mint, user token wallet is stored in Token.
        // Here we are querying the Mints.

        if (rawMintAddrs.length > 30) {
            throw new Error('Too many token addresses in a row')
        }

        let tokens = await Promise.all(rawMintAddrs.map(a => this._getAssetAsToken(address(a))))

        return tokens.filter((t): t is Token => t !== undefined)
    }

    /**
     * List tokens.
     */
    async listTokenBalance(rawWalletAddr: string, rawMintAddrs: string[]): Promise<TokenBalance[]> {
        if (rawMintAddrs.length > 30) {
            throw new Error('Too many token addresses in a row')
        }
        const walletAddr = address(rawWalletAddr)
        const mintAddrs = rawMintAddrs.map(a => address(a))
        const rpc = this.#rpc

        const queryBalanceTasks = mintAddrs.map(async mintAddr => {
            if (mintAddr == NATIVE_MINT) {
                const resp = await rpc.getBalance(walletAddr).send()
                if (resp !== null) {
                    return {
                        addr: mintAddr,
                        amount: resp.value,
                    }
                } else {
                    return {
                        addr: mintAddr,
                        amount: 0n,
                    }
                }
            } else {
                const [ata] = await SPL.findAssociatedTokenPda({
                    owner: walletAddr,
                    tokenProgram: SPL.TOKEN_PROGRAM_ADDRESS,
                    mint: mintAddr,
                })
                const token = await SPL.fetchMaybeToken(this.#rpc, ata)
                if (token.exists) {
                    return {
                        addr: mintAddr,
                        amount: token.data.amount,
                    }
                } else {
                    return {
                        addr: mintAddr,
                        amount: 0n,
                    }
                }
            }
        })

        return await Promise.all(queryBalanceTasks)
    }

    async getNft(addr: Address): Promise<Nft | undefined> {
        const resp = await this.#dasRpc.getAsset(addr).send()

        if ('result' in resp) {
            const item = resp.result
            const collection = item.grouping.find(g => g.group_key === 'collection')?.group_value
            const image = item.content.links?.['image'] as string | undefined

            if (image !== undefined) {
                const nft: Nft = {
                    addr: item.id,
                    collection: collection,
                    image,
                    metadata: item.content.metadata,
                    name: item.content.metadata.name,
                    symbol: item.content.metadata.symbol,
                }
                return nft
            } else {
                console.warn('Ignore nft %s as not image found', item.id)
            }
        }
        return undefined
    }

    async listNfts(rawWalletAddr: string): Promise<Nft[]> {
        const walletAddr = address(rawWalletAddr)
        const resp = await this.#dasRpc
            .getAssetsByOwner({
                ownerAddress: walletAddr,
            })
            .send()
        let result: Nft[] = []

        if ('result' in resp) {
            const assetsResp = resp.result
            for (const item of assetsResp.items) {
                const collection = item.grouping.find(g => g.group_key === 'collection')?.group_value
                const image = item.content.links?.['image'] as string | undefined

                if (image !== undefined) {
                    const nft: Nft = {
                        addr: item.id,
                        collection: collection,
                        image,
                        metadata: item.content.metadata,
                        name: item.content.metadata.name,
                        symbol: item.content.metadata.symbol,
                    }
                    result.push(nft)
                } else {
                    console.warn('Ignore nft %s as not image found', item.id)
                }
            }
        }

        return result
    }

    async _getMultiGameStates(gameAccountKeys: Address[]): Promise<Array<GameState | undefined>> {
        const accounts = await this.#rpc.getMultipleAccounts(gameAccountKeys).send()
        const ret: Array<GameState | undefined> = []
        for (let i = 0; i < accounts.value.length; i++) {
            const key = gameAccountKeys[i]
            const accountInfo = accounts.value[i]
            if (accountInfo !== null) {
                try {
                    ret.push(GameState.deserialize(base64ToUint8Array(accountInfo.data[0])))
                    console.debug('Found game account %s', key)
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

    async _getMultiPlayerStates(profileAccountKeys: Address[]): Promise<Array<PlayerState | undefined>> {
        const accounts = await this.#rpc.getMultipleAccounts(profileAccountKeys).send()
        const ret: Array<PlayerState | undefined> = []
        for (let i = 0; i < accounts.value.length; i++) {
            const key = profileAccountKeys[i]
            const accountInfo = accounts.value[i]
            if (accountInfo !== null) {
                try {
                    ret.push(PlayerState.deserialize(base64ToUint8Array(accountInfo.data[0])))
                    console.info('Found player profile %s', key)
                } catch (_: any) {
                    ret.push(undefined)
                    console.warn('Skip invalid player profile %s', key)
                }
            } else {
                ret.push(undefined)
                console.warn('Player profile %s not exist', key)
            }
        }
        return ret
    }

    // This function returns the account data in Uint8Array which is parsed from base64 string
    // format.
    async _getFinializedBase64AccountData(addr: Address): Promise<Readonly<Uint8Array> | undefined> {
        const value = (await this.#rpc.getAccountInfo(addr, { commitment: 'finalized', encoding: 'base64' }).send())
            .value
        if (value == null) {
            return undefined
        } else {
            return base64ToUint8Array(value.data[0])
        }
    }

    async _getGameState(gameAccountKey: Address): Promise<GameState | undefined> {
        const data = await this._getFinializedBase64AccountData(gameAccountKey)
        if (data !== undefined) {
            return GameState.deserialize(data)
        } else {
            return undefined
        }
    }

    async _getRecipientState(recipientKey: Address): Promise<RecipientState | undefined> {
        const data = await this._getFinializedBase64AccountData(recipientKey)
        if (data !== undefined) {
            return RecipientState.deserialize(data)
        } else {
            return undefined
        }
    }

    async _getRegState(regKey: Address): Promise<RegistryState | undefined> {
        const data = await this._getFinializedBase64AccountData(regKey)
        if (data !== undefined) {
            return RegistryState.deserialize(data)
        } else {
            return undefined
        }
    }

    async _getServerState(serverKey: Address): Promise<ServerState | undefined> {
        const data = await this._getFinializedBase64AccountData(serverKey)
        if (data !== undefined) {
            return ServerState.deserialize(data)
        } else {
            return undefined
        }
    }
}
async function sendTransaction<T, E>(
    signer: TransactionSendingSigner,
    tx: TransactionMessageWithFeePayerAndBlockhashLifetime,
    response: ResponseHandle<T, E>,
    config?: SendTransactionOptions
): Promise<SendTransactionResult<Signature>> {
    response.waitingWallet()

    let transaction: Transaction = compileTransaction(tx)

    try {
        if (config?.signers !== undefined) {
            console.info('Signers: ', config?.signers)
            transaction = await partiallySignTransaction(
                config.signers.map(s => s.keyPair),
                transaction
            )
        }

        const signatures = await signer.signAndSendTransactions([transaction])

        console.log('Signatures:', signatures)

        const signature = getBase58Decoder().decode(signatures[0]) as Signature

        console.info(`Transaction signature: ${signature}`)

        response.confirming(signature)

        return { ok: signature }
    } catch (e: any) {
        console.error(e)
        response.userRejected(e.toString())
        return { err: e }
    }
}

async function confirmSignature<T, E>(
    rpc: Rpc<SolanaRpcApi>,
    signature: Signature,
    response: ResponseHandle<T, E>,
    data: T
) {
    let err: string = 'Unknown'

    for (let i = 0;; i++) {
        await new Promise(r => setTimeout(r, 1000))

        const resp = await rpc.getSignatureStatuses([signature], { searchTransactionHistory: true }).send()

        console.log('Signature response:', resp)

        if (resp.value.length === 0) {
            if (i === MAX_CONFIRM_TIMES) {
                err = 'Transaction signature status not found'
                break
            } else {
                continue
            }
        }

        const status = resp.value[0]

        if (status === null) {
            if (i === MAX_CONFIRM_TIMES) {
                err = 'Transaction signature status not found'
                break
            } else {
                continue
            }
        }

        if (status.err !== null) {
            if (i == MAX_CONFIRM_TIMES) {
                err = status.err.toString()
                break
            } else {
                continue
            }
        }

        if (status.confirmationStatus == null) {
            if (i == MAX_CONFIRM_TIMES) {
                err = 'Transaction confirmation status not found'
                break
            } else {
                continue
            }
        } else {
            return response.succeed(data)
        }
    }

    return response.transactionFailed(err)
}

async function makeTransaction(
    rpc: Rpc<SolanaRpcApi>,
    feePayer: TransactionSendingSigner,
    instructions: TransactionMessage['instructions'][number][]
): Promise<Result<TransactionMessageWithFeePayerAndBlockhashLifetime, string>> {
    const d = new Date()
    let latestBlockhash: Readonly<{ blockhash: Blockhash; lastValidBlockHeight: bigint }>
    try {
        latestBlockhash = (await rpc.getLatestBlockhash().send()).value
    } catch (e: any) {
        return { err: 'block-not-found' }
    }
    if (!latestBlockhash) {
        return { err: 'block-not-found' }
    }

    console.debug(
        'Got block hash %s, took %s milliseconds',
        latestBlockhash.blockhash,
        new Date().getTime() - d.getTime()
    )
    const transactionMessage = pipe(
        createTransactionMessage({ version: 0 }),
        tx => (
            console.info(feePayer, 'Setting the transaction fee payer'),
            setTransactionMessageFeePayer(feePayer.address, tx)
        ),
        tx => (
            console.info(latestBlockhash, 'Setting the transaction lifetime'),
            setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx)
        ),
        tx => (
            console.info(instructions, 'Setting the transaction instructions'),
            appendTransactionMessageInstructions(instructions, tx)
        )
    )

    console.info(transactionMessage, 'Transaction Message')
    return { ok: transactionMessage }
}
