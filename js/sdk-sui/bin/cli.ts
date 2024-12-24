// We use this script to serialize raw data into an array

import { SuiTransport } from '../src/sui-transport';
import { LocalSuiWallet } from '../src/local-wallet'
import { GAME_OBJECT_TYPE, GAS_BUDGET, MAXIMUM_TITLE_LENGTH, PACKAGE_ID, PROFILE_TABLE_ID } from '../src/constants'
// import { ResponseHandle } from '../../sdk-core/src/response'
import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenWithBalance } from "@race-foundation/sdk-core";
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
const wallet = new LocalSuiWallet('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2');

function testCreatePlayerProfile() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const params = {
    nick: 'yuumi Game', // nick string
    pfp: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192', // pfp address params
  }

  let response = new ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>()
  suiTransport.createPlayerProfile(wallet, params, response);
}
async function testGetPlayerProfile() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let res = await suiTransport.getPlayerProfile(wallet.walletAddr)
  console.log('res', res)
}

function testCreateGameAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  // console.log(wallet.walletAddr)
  const params = {
    title: 'yuumi Game', // title string
    bundleAddr: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192', // bundle_addr address params
    owner: wallet.walletAddr, // owner address wallet
    recipientAddr: randomPublicKey(), // recipient_addr address params
    tokenAddr: "0x2", // token_addr address params "0x2"
    maxPlayers: 6, // max_players u64 params
    data_len: 2, // data_len u32 params
    data: Uint8Array.from([1, 2]), // data vector<u8> params
    // entryType: {
    //   kind: 'cash' as const,
    //   minDeposit: BigInt(0),
    //   maxDeposit: BigInt(1000000)
    // },
    // entryType: {
    //   kind: 'ticket' as const,
    //   amount: BigInt(1000000)
    // },
    // entryType: {
    //   kind: 'gating' as const,
    //   collection: 'abc'
    // },
    entryType: {
      kind: 'disabled' as const,
    },
    registrationAddr: '12',
  }

  let response = new ResponseHandle<CreateGameResponse, CreateGameError>()
  suiTransport.createGameAccount(wallet, params, response);
  console.log(response)
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

async function testListTokens() {
  const suiTransport = new SuiTransport('https://fullnode.mainnet.sui.io:443');
  // const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const tokenAddrs = [
    '0xb231fcda8bbddb31f2ef02e6161444aec64a514e2c89279584ac9806ce9cf037::coin::COIN',
    '0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN',
  ]
  let res = await suiTransport.listTokens(tokenAddrs);
  console.log('tokens', res)
}


async function testListTokensWithBalance() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  // const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const tokenAddrs = [
    '0xb231fcda8bbddb31f2ef02e6161444aec64a514e2c89279584ac9806ce9cf037::coin::COIN',
    '0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN',
  ]
  // const walletAddr = '0x5b6eb18e764749862726832bf35e37d597975d234ef341fb39770a736879bc7b'
  const walletAddr = '0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e'
  let res = await suiTransport.listTokensWithBalance(walletAddr, tokenAddrs);
  console.log('tokens', res)
}
async function testGetGameAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const objectId = '0x0cb1411ec1ed736640a53acf29b22559efd869429b82915123b600c741fe2add'
  let res = await suiTransport.getGameAccount(objectId);
  console.log('testGetGameAccount', res)
}

async function testGetRegistration() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const objectId = '0xedc53aecfdf417d6a57d5c2cbf8ad30f877d39cc67ed77ddf5e0ad7a8827d15c'
  let res = await suiTransport.getRegistration(objectId);
  console.log('testGetRegistration', res)
}


async function testServerAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const objectId = '0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e'
  let res = await suiTransport.getServerAccount(objectId);
  console.log('testServerAccount', res)
}


async function testCreateRecipient() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let response = new ResponseHandle<CreateRecipientResponse, CreateRecipientError>()
  let params: CreateRecipientParams = {
    capAddr: '0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192',
    slots: [
      {
        id: 0,
        slotType: 'token', // nft or token
        tokenAddr: '0x2::sui::SUI',
        initShares: [
          { owner: { identifier: 'Race1' }, weights: 10 },
          { owner: { identifier: 'Race2' }, weights: 20 },
        ]
      },
      // {
      //   id: 1,
      //   slotType: 'token', // nft or token
      //   tokenAddr: '0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e',
      //   initShares: [
      //     { owner: { identifier: 'Race'}, weights: 10 },
      //     { owner: { identifier: 'Race'}, weights: 20 },
      //   ]
      // }
    ]
  }
  const wallet = new LocalSuiWallet('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2');

  let res = await suiTransport.createRecipient(wallet, params, response);
  console.log('testCreateRecipient', res)
}


function main() {
  const args = process.argv.slice(2, process.argv.length);
  switch (args[0] || '') {
    case 'getServerAccount':
      testServerAccount()
      break
    case 'getRegistration':
      testGetRegistration()
      break
    case 'createGameAccount':
      testCreateGameAccount()
      break
    case 'createPlayerProfile':
      testCreatePlayerProfile()
      break
    case 'getPlayerProfile':
        testGetPlayerProfile()
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
    case 'getListTokens':
      testListTokens()
      break
    case 'getListTokensWithBalance':
      testListTokensWithBalance()
      break
    case 'getGameAccount':
      testGetGameAccount()
      break
    case 'createRecipient':
      testCreateRecipient()
      break

    default:
      console.error('Invalid command')
  }
}

main()

function randomPublicKey(): string {
  return Ed25519Keypair.generate().getPublicKey().toSuiAddress()
}