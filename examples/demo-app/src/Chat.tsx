import { AppClient } from 'race-sdk';
import { useContext, useEffect, useState } from 'react';
import { CHAIN, RPC } from './constants';
import GameContext from './game-context';

interface Message {
  sender: string,
  text: string,
}

interface State {
  messages: Message[],
  num_of_clients: number,
  num_of_servers: number,
}

function Chat(props: { profile: any, account: any, state: State }) {

  const { state } = props;
  const { client } = useContext(GameContext);
  const [text, setText] = useState<string>('');

  const sendMessage = async () => {
    if (text.length > 0) {
      client && await client.submit_event({ PublicMessage: { text } });
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

  if (state.messages === undefined) return null;

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
