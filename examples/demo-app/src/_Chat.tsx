import { AppClient, GameEvent } from '@race/sdk-core';
import { useParams } from 'react-router-dom';
import { useContext, useEffect, useState } from 'react';
import { CHAIN_TO_RPC } from './constants';
import { LogsContext } from './logs-context';
import { ProfileContext } from './profile-context';
import { useGameContext } from './App';
import { useWallet, createTransport } from './integration';

interface Message {
    sender: string,
    text: string,
}

interface State {
    messages: Message[],
}

function Chat() {
    let [state, setState] = useState<State | undefined>(undefined);
    let [client, setClient] = useState<AppClient | undefined>(undefined);
    const { addLog } = useContext(LogsContext);
    let { addr } = useParams();
    let { chain } = useGameContext();
    let profile = useContext(ProfileContext);
    let wallet = useWallet(chain);
    const [text, setText] = useState<string>('');

    // Game event handler
    const onEvent = (_context: any, state: Uint8Array, event: GameEvent | undefined) => {
        // if (event !== null) {
        //     addLog(event);
        // }
        // setState(state);
    }

    // Button callback to join the raffle
    const onJoin = async () => {
        // if (client !== undefined) {
        //     await client.join(1n);
        // }
    }

    // Initialize app client
    useEffect(() => {
        const initClient = async () => {
            if (profile !== undefined && addr !== undefined) {
                console.log("Create AppClient");
                let rpc = CHAIN_TO_RPC[chain];
              let transport = createTransport(chain, rpc);
              let client = await AppClient.initialize(transport, wallet, addr, onEvent);
                setClient(client);
                await client.attachGame();
            }
        };
        initClient();
    }, [profile, addr]);


    const sendMessage = async () => {
        if (text.length > 0) {
            // client && await client.submitEvent({ PublicMessage: { text } });
            setText('');
        }
    }

    const onChangeText = (e: React.ChangeEvent<HTMLInputElement>) => {
        setText(e.target.value);
    }

    const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            sendMessage();
        }
    }

    if (state === undefined) return null;

    return <div className="h-full w-full flex flex-col p-4">
        <div className="flex-1 relative">
            <div className="absolute inset-0 overflow-scroll">
                {
                    state.messages.map((msg, idx) => (
                        <div key={idx} className="p-2">
                            <div className="flex">
                                <div className="font-bold rounded-lg p-2 text-sm">{msg.sender}</div>
                            </div>
                            <div className="p-2">{msg.text}</div>
                        </div>
                    ))
                }
            </div>
        </div>

        <div className="flex flex-row w-full h-12">
            <div className="box-border border border-black overflow-hidden flex-1 flex">
                <input className="box-border flex-1 h-full outline-none text-center" name="message-text" type="text" value={text} onChange={onChangeText} onKeyDown={onKeyDown} />
                <button className="box-border h-full px-4 bg-black text-white" onClick={sendMessage}>Send</button>
            </div>
        </div>
    </div>
}

export default Chat;
