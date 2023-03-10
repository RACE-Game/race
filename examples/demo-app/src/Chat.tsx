import { AppClient, Event } from 'race-sdk';
import { useParams } from 'react-router-dom';
import { useContext, useEffect, useRef, useState } from 'react';
import { CHAIN, RPC } from './constants';
import LogsContext from './logs-context';
import ProfileContext from './profile-context';
import Header from './Header';

interface Message {
  sender: string,
  text: string,
}

interface State {
  messages: Message[],
}

function Chat() {

  let [state, setState] = useState<State | undefined>(undefined);
  let client = useRef<AppClient | undefined>(undefined);
  const { addLog, clearLog } = useContext(LogsContext);
  let { addr } = useParams();
  let profile = useContext(ProfileContext);
  const [text, setText] = useState<string>('');

  // Game event handler
  const onEvent = (context: any, state: State, event: Event | undefined) => {
    if (event !== undefined) {
      addLog(event);
    }
    setState(state);
  }

  // Initialize app client
  useEffect(() => {
    const initClient = async () => {
      if (profile !== undefined && addr !== undefined) {
        console.log("Create AppClient");
        let c = await AppClient.try_init(CHAIN, RPC, profile.addr, addr, onEvent);
        client.current = c;
        await c.attach_game();
      }
    };
    initClient();
    return () => {
      clearLog();
      if (client.current) {
        client.current.close();
      }
    }
  }, [profile, addr]);

  const sendMessage = async () => {
    if (text.length > 0) {
      client.current && await client.current.submit_event({ Message: { text } });
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

  if (addr == undefined || state === undefined) {
    return null;
  }

  return <div className="h-full w-full flex flex-col p-4">
    <div className="flex-1 relative">
      <Header gameAddr={addr} />
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
