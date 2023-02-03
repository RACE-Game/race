import { AppClient } from 'race-sdk';
import { useEffect, useState } from 'react';
import { CHAIN, RPC } from './constants';

interface State {
  messages: string[],
  num_of_clients: number,
  num_of_servers: number,
}

function Chat(props: { profile: any, account: any }) {

  const { profile, account } = props;

  const [state, setState] = useState<State>({
    messages: [],
    num_of_clients: 0,
    num_of_servers: 0
  });

  const [client, setClient] = useState<AppClient | undefined>(undefined);

  const onEvent = (addr: string, context: any, state: State) => {
    console.log(addr, context, state);
    setState(state);
  }

  const sendMessage = async () => {
    client && await client.submit_event({ PublicMessage: { text: "Hello!" } })
  }

  const join = async () => {
    client && client.join(0, 0n);
  }

  useEffect(() => {
    if (client === undefined && account !== undefined && profile !== undefined) {
      AppClient.try_init(CHAIN, RPC, profile.addr, account.addr, onEvent).then(client => {
        setClient(client);
        client.attach_game().then(() => {
          client.join(0, 0n);
        })
      });
    }
  }, [client, profile, account]);

  return <div className="p-4">
    <div> Client: {state.num_of_clients} | Servers: {state.num_of_servers} </div>
    <ul>
      {
        state.messages.map(msg => (<li>{msg}</li>))
      }
    </ul>
    <button className="m-2 px-4 py-2 bg-black text-white" onClick={join}>Join</button>
    <button className="m-2 px-4 py-2 bg-black text-white" onClick={sendMessage}>Send</button>
  </div>
}

export default Chat;
