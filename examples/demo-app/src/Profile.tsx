import React, { useContext, useState, useEffect } from 'react';
import HelperContext from './helper-context';
import ProfileContext, { ProfileData } from './profile-context';
import { useWallet } from '@solana/wallet-adapter-react';
import { SolanaWalletAdapter } from 'race-sdk-solana';

function Profile(props: { updateProfile: (profile: ProfileData) => void }) {
    let [nick, setNick] = useState<string>("");
    let helper = useContext(HelperContext);
    let profile = useContext(ProfileContext);
    let wallet = useWallet();
    let walletAdapter = new SolanaWalletAdapter(wallet);

    const editNick = (e: React.ChangeEvent<HTMLInputElement>) => {
        setNick(e.target.value);
    }

    const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            setNick(e.currentTarget.value);
        }
    }

    useEffect(() => {
        if (helper !== undefined && walletAdapter.isConnected) {
            const q = async () => {
                if (helper !== undefined && walletAdapter.isConnected) {
                    const profile = await helper.get_profile(walletAdapter.walletAddr);
                    if (profile !== undefined) {
                        props.updateProfile(profile);
                        setNick(profile.nick);
                    }
                }
            }
            q();
        }
    }, [helper, walletAdapter.isConnected && walletAdapter.walletAddr]);

    const createProfile = async () => {
        if (helper !== undefined) {
            if (nick === "") {
                alert("Profile name can't be empty");
            } else {
                console.log("Wallet:", wallet);
                const walletAdapter = new SolanaWalletAdapter(wallet);
                await helper.create_profile(walletAdapter, nick, "");
                const profile = await helper.get_profile(walletAdapter.walletAddr);
                props.updateProfile(profile);
            }
        }
    }

    return (
        <div className="grid place-items-center border border-gray-500 rounded-lg">
            <input
                className="text-gray-800 focus:text-black bg-transparent border-b border-black outline-none text-center p-4 text-lg"
                name="nick"
                type="text"
                placeholder="Enter nick"
                disabled={helper === undefined || profile !== undefined}
                onChange={editNick}
                onKeyDown={onKeyDown}
                value={nick} />

            {
                profile !== undefined ?
                    <div className="text-gray-500">
                        Connected
                    </div> :
                    <button className="px-4 py-2 rounded-lg border border-black hover:bg-gray-100 active:bg-gray-200 transition-all active:translate-y-1"
                        onClick={createProfile}>
                        Create Profile
                    </button>
            }
        </div >
    )
}

export default Profile;
