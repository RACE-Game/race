import { IWallet } from './wallet';
import { GameAccount, GameBundle, ServerAccount, PlayerProfile, VoteType, RegistrationAccount } from './accounts';
export type CreateGameAccountParams = {
    title: string;
    bundleAddr: string;
    tokenAddr: string;
    maxPlayers: number;
    minDeposit: bigint;
    maxDeposit: bigint;
    data: Uint8Array;
};
export type CloseGameAccountParams = {
    gameAddr: string;
};
export type JoinParams = {
    gameAddr: string;
    amount: bigint;
    accessVersion: bigint;
    position: number;
};
export type DepositParams = {
    playerAddr: string;
    gameAddr: string;
    amount: bigint;
    settleVersion: bigint;
};
export type VoteParams = {
    gameAddr: string;
    voteType: VoteType;
    voterAddr: string;
    voteeAddr: string;
};
export type CreatePlayerProfileParams = {
    nick: string;
    pfp?: string;
};
export type PublishGameParams = {
    uri: string;
    name: string;
    symbol: string;
};
export type CreateRegistrationParams = {
    isPrivate: boolean;
    size: number;
};
export type RegisterGameParams = {
    gameAddr: string;
    regAddr: string;
};
export type UnregisterGameParams = {
    gameAddr: string;
    regAddr: string;
};
export interface ITransport {
    createGameAccount(wallet: IWallet, params: CreateGameAccountParams): Promise<string>;
    closeGameAccount(wallet: IWallet, params: CloseGameAccountParams): Promise<void>;
    join(wallet: IWallet, params: JoinParams): Promise<void>;
    deposit(wallet: IWallet, params: DepositParams): Promise<void>;
    vote(wallet: IWallet, params: VoteParams): Promise<void>;
    createPlayerProfile(wallet: IWallet, params: CreatePlayerProfileParams): Promise<string>;
    publishGame(wallet: IWallet, params: PublishGameParams): Promise<string>;
    createRegistration(wallet: IWallet, params: CreateRegistrationParams): Promise<string>;
    registerGame(wallet: IWallet, params: RegisterGameParams): Promise<void>;
    unregisterGame(wallet: IWallet, params: UnregisterGameParams): Promise<void>;
    getGameAccount(addr: string): Promise<GameAccount | undefined>;
    getGameBundle(addr: string): Promise<GameBundle | undefined>;
    getPlayerProfile(addr: string): Promise<PlayerProfile | undefined>;
    getServerAccount(addr: string): Promise<ServerAccount | undefined>;
    getRegistration(addr: string): Promise<RegistrationAccount | undefined>;
}
//# sourceMappingURL=transport.d.ts.map