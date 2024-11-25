// We use this script to serialize raw data into an array

import { SuiTransport } from '../src/sui-transport';
import { LocalSuiWallet } from '../src/local-wallet'
// import { ResponseHandle } from '../../sdk-core/src/response'
import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance } from "@race-foundation/sdk-core";

function main() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const localSuiWallet = new LocalSuiWallet();
  const wallet = {
    isConnected: localSuiWallet.isConnected,
    walletAddr: localSuiWallet.walletAddr,
    wallet: localSuiWallet.wallet
  }
  const params = {
    title: 'yuumi Ganme', // title string
    bundleAddr: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192', // bundle_addr address params
    owner: localSuiWallet.walletAddr, // owner address wallet
    recipientAddr: 'recipient_addr', // recipient_addr address params
    tokenAddr: "0x2", // token_addr address params "0x2"
    maxPlayers: 6, // max_players u64 params
    data_len: 2, // data_len u32 params
    data: Uint8Array.from([1,2]), // data vector<u8> params
    entryType: {
      kind: 'cash' as const,
      minDeposit: BigInt(0),
      maxDeposit: BigInt(1000000)
    },
    registrationAddr: '12',
  }
  
  let response = new ResponseHandle<CreateGameResponse, CreateGameError>()
  suiTransport.createGameAccount(wallet, params, response);
}

main()

