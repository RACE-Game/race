import React, { useContext, useState, useEffect, FC } from 'react';
import { HelperContext } from './helper-context';
import { ProfileContext } from './profile-context';
import { PlayerProfile } from '@race/sdk-core';
import { useWallet } from './integration';
import { Chain } from './types';

type ProfileProps = {
    chain: Chain;
    updateProfile: (profile: PlayerProfile) => void;
}

const Profile: FC<ProfileProps> = ({ chain, updateProfile }) => {
    let [nick, setNick] = useState<string>("");
    let helper = useContext(HelperContext);
    let profile = useContext(ProfileContext);
    let wallet = useWallet(chain);

    const editNick = (e: React.ChangeEvent<HTMLInputElement>) => {
        setNick(e.target.value);
    }

    const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            setNick(e.currentTarget.value);
        }
    }

    useEffect(() => {
        if (helper !== undefined && wallet.isConnected) {
            const q = async () => {
                if (helper !== undefined && wallet.isConnected) {
                    console.log("TSX: wallet addr = ", wallet.walletAddr)
                    const profile = await helper.getProfile(wallet.walletAddr);
                    if (profile !== undefined) {
                        updateProfile(profile);
                        setNick(profile.nick);
                    }
                }
            }
            q();
        }
    }, [helper, wallet.isConnected && wallet.walletAddr]);

    const createProfile = async () => {
        if (helper !== undefined) {
            if (nick === "") {
                alert("Profile name can't be empty");
            } else {
                console.log("Wallet:", wallet);
                await helper.createProfile(wallet, nick, undefined);
                const profile = await helper.getProfile(wallet.walletAddr);
                if (profile !== undefined) {
                    updateProfile(profile);
                }
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
