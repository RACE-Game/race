// We use this script to serialize raw data into an array

import { SuiTransport } from '../src/sui-transport';
import { LocalSuiWallet } from '../src/local-wallet'
import { GAME_OBJECT_TYPE, GAS_BUDGET, MAXIMUM_TITLE_LENGTH, PACKAGE_ID, PROFILE_TABLE_ID } from '../src/constants'
// import { ResponseHandle } from '../../sdk-core/src/response'
import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance } from "@race-foundation/sdk-core";

function testCreatePlayerProfile() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const wallet = new LocalSuiWallet('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2');
  const params = {
    nick: 'yuumi Game', // nick string
    pfp: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192', // pfp address params
  }

  let response = new ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>()
  suiTransport.createPlayerProfile(wallet, params, response);
}

function testCreateGameAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const wallet = new LocalSuiWallet('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2');
  const params = {
    title: 'yuumi Game', // title string
    bundleAddr: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192', // bundle_addr address params
    owner: wallet.walletAddr, // owner address wallet
    recipientAddr: 'recipient_addr', // recipient_addr address params
    tokenAddr: "0x2", // token_addr address params "0x2"
    maxPlayers: 6, // max_players u64 params
    data_len: 2, // data_len u32 params
    data: Uint8Array.from([1, 2]), // data vector<u8> params
    // entryType: {
    //   kind: 'cash' as const,
    //   minDeposit: BigInt(0),
    //   maxDeposit: BigInt(1000000)
    // },
    entryType: {
      kind: 'ticket' as const,
      amount: BigInt(1000000)
    },
    registrationAddr: '12',
  }

  let response = new ResponseHandle<CreateGameResponse, CreateGameError>()
  suiTransport.createGameAccount(wallet, params, response);
}

async function testGetToken() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let res = await suiTransport.getToken('0x5d9865999eb9a4a5d7cb6615260e42c6400aec1b34cfbb2070005925e673e92e::deliver::GalxeNFT');
  console.log(res)
}

async function testGetNFT() {
  const suiTransport = new SuiTransport('https://fullnode.mainnet.sui.io:443');
  // const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let res = await suiTransport.getNft('0x476194da0dbd8a0241cdf05f4eb52ba1bad8e77a5c141b2e61b2d0d246aa8fcd');
  console.log(res)
}
async function testGetNFTLIST() {
  const suiTransport = new SuiTransport('https://fullnode.mainnet.sui.io:443');
  // const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let res = await suiTransport.listNfts('0x5b6eb18e764749862726832bf35e37d597975d234ef341fb39770a736879bc7b');
  console.log('res', res)
}

function main() {
  const args = process.argv.slice(2, process.argv.length);
  switch (args[0] || '') {
    case 'createGameAccount':
      testCreateGameAccount()
      break
    case 'createPlayerProfile':
      testCreatePlayerProfile()
      break
    case 'getToken':
      testGetToken()
      break
    case 'getNFT':
      testGetNFT()
      break
    case 'getNFTLIST':
      testGetNFTLIST()
      break
    default:
      console.error('Invalid command')
  }
}

main()
