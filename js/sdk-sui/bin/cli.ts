// We use this script to serialize raw data into an array

import { SuiTransport } from '../src/sui-transport';
import { LocalSuiWallet } from '../src/local-wallet'
import { GAME_OBJECT_TYPE, GAS_BUDGET, MAXIMUM_TITLE_LENGTH, PACKAGE_ID, PROFILE_TABLE_ID } from '../src/constants'
// import { ResponseHandle } from '../../sdk-core/src/response'
import { CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, Nft, IStorage, Token, ITransport, IWallet, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegistrationAccount, RegistrationWithGames, ServerAccount, SendTransactionResult, UnregisterGameParams, VoteParams, ResponseHandle, CreateGameResponse, CreateGameError, CreatePlayerProfileError, CreatePlayerProfileResponse, CreateRecipientError, CreateRecipientParams, CreateRecipientResponse, DepositError, DepositResponse, JoinError, JoinResponse, RecipientClaimError, RecipientClaimResponse, RegisterGameError, RegisterGameResponse, TokenBalance } from "@race-foundation/sdk-core";
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
const wallet = new LocalSuiWallet('suiprivkey1qqds4vhlnm38pma946w5ke4g2846wpkgfygu88auscspswd5d4hl6fvc4q2');

const TEST_PACKAGE_ID = "0x094ab410b77496fc9ddccc9f330e2495583df5e6ea59e4498fff5a6172eac462";
const TEST_CASH_GAME_ID = "0x5d5e5b48ba5decc365a46777ad20e4ed926e3b6fb38c5fd06729a999496c0c6a";
const TEST_TICKET_GAME_ID = "0xcfc82be4212e504a2bc8b9a6b5b66ed0db92be4e2ab0befe5ba7146a59f54665"
const TEST_RECIPIENT_ID = "0x8b8e76d661080e47d76248cc33b43324b4126a8532d7642ab6c47946857c1e1c";
const TEST_REGISTRY_ID = "0xcb430f98bd97f8c3697cbdbf0de6b9b59411b2634aeebd07f4434fec30f443c7";
const TEST_GAME_NFT = "0x5ebed419309e71c1cd28a3249bbf792d2f2cc8b94b0e21e45a9873642c0a5cdc";

function testCreatePlayerProfile() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const params = {
    nick: 'Race Sui Tester', // nick string
    pfp: undefined, // pfp address params
  }

  let response = new ResponseHandle<CreatePlayerProfileResponse, CreatePlayerProfileError>()
  suiTransport.createPlayerProfile(wallet, params, response);
}

async function testGetPlayerProfile() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  let res = await suiTransport.getPlayerProfile('0x5b6eb18e764749862726832bf35e37d597975d234ef341fb39770a736879bc7b')
  console.log('res', res)
}

async function testCreateGameAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  // console.log(wallet.walletAddr)
  const params = {
    title: 'yuumi Game', // title string
    bundleAddr: TEST_GAME_NFT, // bundle_addr address params
    owner: wallet.walletAddr, // owner address wallet
    recipientAddr: randomPublicKey(), // recipient_addr address params
    tokenAddr: "0x2::sui::SUI", // token_addr address params "0x2"
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
      amount: BigInt(100_000_000)
    },
    // entryType: {
    //   kind: 'gating' as const,
    //   collection: 'abc'
    // },
    // entryType: {
    //   kind: 'disabled' as const,
    // },
    registrationAddr: TEST_REGISTRY_ID,
  }

  let response = new ResponseHandle<CreateGameResponse, CreateGameError>()
  let result = await suiTransport.createGameAccount(wallet, params, response);
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
  let res = await suiTransport.listTokenBalance(walletAddr, tokenAddrs);
  console.log('tokens', res)
}

async function testGetGameAccount() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const objectId = TEST_CASH_GAME_ID;
  let res = await suiTransport.getGameAccount(objectId);
  console.log('testGetGameAccount', res)
}

async function testGetRegistration() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const objectId = '0xedc53aecfdf417d6a57d5c2cbf8ad30f877d39cc67ed77ddf5e0ad7a8827d15c'
  let res = await suiTransport.getRegistration(objectId);
  console.log('testGetRegistration', res)
}

// async function testGetRegistrationWithGames() {
//   const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
//   const objectId = '0x65f80e8f4e82f4885c96ccba4da02668428662e975b0a6cd1fa08b61e4e3a2fc'
//   let res = await suiTransport.getRegistrationWithGames(objectId);
//   console.log('testGetRegistrationWithGames', res)
// }

async function testRegisterGame() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const params: RegisterGameParams = {
    gameAddr: '0x23f5786879d909cfc7c75414b2156b24333e553bedf77c80421e4c8f0bd32968',
    regAddr: '0x251a0ef52ce578f512f76edba4e740a3a1f79e0f94c9a3595bdd24b537191964',
  }
  let response = new ResponseHandle<RegisterGameResponse, RegisterGameError>()
  let res = await suiTransport.registerGame(wallet, params, response);
  console.log('testRegisterGame', response)
}

async function testJoinGame() {
  const suiTransport = new SuiTransport('https://fullnode.devnet.sui.io:443');
  const params: JoinParams = {
    gameAddr: TEST_CASH_GAME_ID,
    amount: BigInt(100_000_000),
    position: 3,
    verifyKey: 'player3',
  };
  let response = new ResponseHandle<JoinResponse, JoinError>();
  let res = await suiTransport.join(wallet, params, response);
  console.log('testJoinGame', response);
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
    // case 'getRegistrationWithGames':
    //   testGetRegistrationWithGames()
    //   break
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
    case 'registerGame':
      testRegisterGame()
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
