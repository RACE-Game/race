import { useState, useEffect } from 'react'
import { Outlet, useOutletContext, useNavigate } from 'react-router-dom';
import Sidemenu from './Sidemenu';
import Profile from './Profile';
import init, { AppHelper, Event } from 'race-sdk';
import './App.css'
import { ProfileContext, ProfileData } from './profile-context';
import { LogsContext } from './logs-context';
import { HelperContext } from './helper-context';
import Logs from './Logs';
import { CHAIN_TO_RPC } from './constants';
import { Chain, isChain } from './types';
import { createTransport, getWalletWrapper } from './integration';

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
        <div className="row-span-6 col-span-2">
            <Outlet context={{ chain }} />
        </div>
        <Profile updateProfile={props.setProfile} chain={chain} />
        <div className="row-span-5">
            <Logs logs={props.logs} />
        </div>
    </div>
    )
}

function App() {
    const [chain, setChain] = useState<Chain | undefined>(undefined);
    const [helper, setHelper] = useState<AppHelper | undefined>(undefined);
    const [profile, setProfile] = useState<ProfileData | undefined>(undefined);
    let [logs, setLogs] = useState<Array<Event>>([]);
    let nav = useNavigate();

    useEffect(() => {
        nav('/')
    }, []);

    const addLog = (event: Event) => {
        console.log("Add event log:", event);
        setLogs(logs => {
            let newLogs = [...logs, event];
            if (newLogs.length > 30) {
                newLogs.shift();
            }
            return newLogs;
        });
    }

    const clearLog = () => {
        setLogs([])
    };

    useEffect(() => {
        if (chain !== undefined) {
            console.log("Chain: ", chain);
            let rpc = CHAIN_TO_RPC[chain];
            const initHelper = async () => {
                await init();
                let transport = createTransport(chain, rpc);
                let client = await AppHelper.try_init(transport);
                console.log("AppHelper initialized");
                setHelper(client);
            }
            initHelper();
        }
    }, [chain]);

    if (chain === undefined || !isChain(chain)) {
        return <div className="w-full h-screen flex items-center justify-center">
            <select
                className="p-2 bg-white border border-black"
                onChange={(e) => {
                    const value = e.currentTarget.value;
                    if (isChain(value)) {
                        setChain(value);
                    }
                }}>
                <option value="">[Select environment]</option>
                <option value="facade">Facade</option>
                <option value="solana-local">Solana(Local)</option>
                <option value="solana-mainnet">Solana(Mainnet)</option>
            </select>
        </div>
    }

    const WalletWrapper = getWalletWrapper(chain);
    return (
        <HelperContext.Provider value={helper}>
            <ProfileContext.Provider value={profile}>
                <LogsContext.Provider value={{ addLog, clearLog }}>
                    <WalletWrapper>
                        <Content logs={logs} setProfile={setProfile} chain={chain} />
                    </WalletWrapper>
                </LogsContext.Provider>
            </ProfileContext.Provider>
        </HelperContext.Provider>
    );
}

export default App;
