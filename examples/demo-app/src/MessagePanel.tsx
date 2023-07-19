import { AppClient, Message } from '@race-foundation/sdk-core';
import React, { useState } from 'react';

interface MessagePanelProps {
    messages: Message[];
    client: AppClient,
}


function MessagePanel(props: MessagePanelProps) {
    console.log(props);
    let [message, setMessage] = useState<string>('');

    const onEdit = (e: React.ChangeEvent<HTMLInputElement>) => {
        setMessage(e.target.value);
    }

    const onSend = async () => {
        await props.client.submitMessage(message);
        setMessage('');
    }

    return (
        <div className="border border-gray-700 p-2 w-full h-40 flex flex-col">
            <div className="flex-1 flex flex-col overflow-y-scroll">
                {
                    props.messages.map(msg =>
                        <div>
                            <span className="text-black font-bold"> {msg.sender} </span>
                            <span className="text-gray-500"> {msg.content} </span>
                        </div>
                    )
                }

            </div>
            <div className="h-12 flex">
                <input
                    type="text"
                    className="border border-black w-3/4 h-full"
                    value={message}
                    onChange={onEdit} />
                <button
                    className="bg-black text-white w-1/4 h-full"
                    onClick={onSend}
                >
                    Send
                </button>
            </div>
        </div>
    )
}

export default MessagePanel;
