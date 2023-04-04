import React, { useState, useEffect, FC } from 'react'
import { Outlet, useOutletContext } from 'react-router-dom';
import Sidemenu from './Sidemenu';
import Profile from './Profile';
import init, { AppHelper, Event } from 'race-sdk';
import './App.css'
import ProfileContext, { ProfileData } from './profile-context';
import LogsContext from './logs-context';
import HelperContext from './helper-context';
import Header from './Header';
import Logs from './Logs';
import SolanaWalletWrapper from './SolanaWalletWrapper';
import FacadeWalletWrapper from './FacadeWalletWrapper';

type Chain = 'solana' | 'facade';

interface RenderContentProps {
    chain: Chain,
    setProfile: (profile: ProfileData) => void
    logs: Array<Event>
}

interface OutletContextType {
    chain: Chain,
}

export function useGameContext() {
    return useOutletContext<OutletContextType>();
}

const Content = (props: RenderContentProps) => {
    let { chain } = props;
    return (<div className="w-screen max-w-7xl min-h-screen grid grid-cols-4 grid-rows-6 p-4 gap-2">
        <div className="row-span-6">
            <Sidemenu chain={chain} />
        </div>
        <div className="col-span-2">
            <Header />
        </div>
        <Profile updateProfile={props.setProfile} />
        <div className="row-span-6 col-span-2">
            <Outlet context={{ chain }} />
        </div>
        <div className="row-span-5">
            <Logs logs={props.logs} />
        </div>
    </div>
    )
}

function getRpc(chain: string): string {
    switch (chain) {
        case 'solana':
            return 'http://localhost:8899';
        default:
            return 'ws://localhost:12002';
    }
}

function App() {
    const [chain, setChain] = useState<Chain | undefined>(undefined);
    const [helper, setHelper] = useState<AppHelper | undefined>(undefined);
    const [profile, setProfile] = useState<ProfileData | undefined>(undefined);
    let [logs, setLogs] = useState<Array<Event>>([]);

    const addLog = (event: Event) => {
        setLogs(logs => {
            let newLogs = [...logs, event];
            if (newLogs.length > 30) {
                newLogs.shift();
            }
            return newLogs;
        });
    }

    useEffect(() => {
        if (chain !== undefined) {
            console.log("Chain: ", chain);
            let endpoint = getRpc(chain);
            const initHelper = async () => {
                await init();
                let client = await AppHelper.try_init(chain, endpoint);
                console.log("AppHelper initialized");
                setHelper(client);
            }
            initHelper();
        }
    }, [chain]);

    let WalletWrapper = null;
    switch (chain) {
        case 'solana':
            WalletWrapper = SolanaWalletWrapper;
            break;
        case 'facade':
            WalletWrapper = FacadeWalletWrapper;
            break;
    }

    if (WalletWrapper === null || chain === undefined) {
        return <div className="w-full h-screen flex items-center justify-center">
            <select
                className="p-2 bg-white border border-black"
                onChange={(e) => {
                    const value = e.currentTarget.value;
                    if (value === 'facade' || value === 'solana') {
                        setChain(value);
                    }
                }}>
                <option value="">[Select chain]</option>
                <option value="solana">Solana</option>
                <option value="facade">Facade</option>
            </select>
        </div>
    }

    return (
        <HelperContext.Provider value={helper}>
            <ProfileContext.Provider value={profile}>
                <LogsContext.Provider value={{ addLog }}>
                    <WalletWrapper>
                        <Content logs={logs} setProfile={setProfile} chain={chain} />
                    </WalletWrapper>
                </LogsContext.Provider>
            </ProfileContext.Provider>
        </HelperContext.Provider>
    );
}

export default App;
