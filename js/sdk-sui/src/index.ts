import { Balance, getFullnodeUrl, SuiClient } from '@mysten/sui/client';
import { getFaucetHost, requestSuiFromFaucetV1 } from '@mysten/sui/faucet';
import { MIST_PER_SUI } from '@mysten/sui/utils';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';

async function fun() {
    // replace <YOUR_SUI_ADDRESS> with your actual address, which is in the form 0x123...
    // random Keypair
    const keypair = new Ed25519Keypair();
    const publicKey = keypair.getPublicKey();
    // Keypair from an existing secret key (Uint8Array)
    const MY_ADDRESS = publicKey.toSuiAddress();

    // create a new SuiClient object pointing to the network you want to use
    const suiClient = new SuiClient({ url: 'https://fullnode.devnet.sui.io:443' });

    // Convert MIST to Sui
    const balance = (balance:Balance) => {
        return Number.parseInt(balance.totalBalance) / Number(MIST_PER_SUI);
    };

    // store the JSON representation for the SUI the address owns before using faucet
    const suiBefore = await suiClient.getBalance({
        owner: MY_ADDRESS,
    });

    await requestSuiFromFaucetV1({
        // use getFaucetHost to make sure you're using correct faucet address
        // you can also just use the address (see Sui TypeScript SDK Quick Start for values)
        host: getFaucetHost('devnet'),
        recipient: MY_ADDRESS,
    });

    // store the JSON representation for the SUI the address owns after using faucet
    const suiAfter = await suiClient.getBalance({
        owner: MY_ADDRESS,
    });
    // Output result to console.
    console.log(
        `Balance before faucet: ${balance(suiBefore)} SUI. Balance after: ${balance(
            suiAfter,
        )} SUI. Hello, SUI!`,
    );
}
fun()
