import { GameEvent } from '@race/sdk-core';

const replacer = (_key: string, value: any) =>
    typeof value === "bigint" ? value.toString() : value;

function LogItem(props: { event: GameEvent }) {

    let event = props.event;
    if (event === undefined) {
        return null;
    }

    let sender = 'sender' in event ? '' + event.sender : '(SYSTEM)';
    let kind = event.constructor.name;
    let data = JSON.stringify(event, replacer);

    return (
        <div className="p-2 mb-2 flex-col items-stretch w-full border border-black rounded-lg">
            <div className="font-bold flex-1">
                {kind}
            </div>
            <div className="text-xs text-gray-500 text-right">
                {sender}
            </div>
            <div className="text-xs p-1 bg-gray-100 rounded-sm whitespace-normal break-all">
                {data}
            </div>
        </div>

    );
}

function Logs(props: { logs: Array<GameEvent> }) {
    return (
        <div className="h-full w-full relative">
            <div className="absolute p-4 rounded-lg border border-gray-500 inset-0 overflow-y-scroll">
                <h4 className="font-bold">Events:</h4>

                <div className="flex flex-col-reverse">
                    {
                        props.logs.map((e, i) => (
                            <LogItem key={i} event={e} />
                        ))
                    }
                </div>
            </div>
        </div>
    );
}

export default Logs;