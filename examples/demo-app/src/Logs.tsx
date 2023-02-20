import { Event } from 'race-sdk';


const replacer = (_key: string, value: any) =>
  typeof value === "bigint" ? value.toString() : value;

function LogItem(props: { event: Event }) {

  let event = props.event;
  let sender = event.sender();
  let kind = event.kind();
  let data = JSON.stringify(event.data(), replacer);

  return (
    <div className="p-2 mb-2 flex-col items-stretch w-full border border-black rounded-lg">
      <div className="font-bold flex-1">
        {kind}
      </div>
      <div className="text-xs text-gray-500 text-right">
        {sender ? sender : "(SYSTEM)"}
      </div>
      <div className="text-xs p-1 bg-gray-100 rounded-sm whitespace-normal break-all">
        {data}
      </div>
    </div>

  );
}

function Logs(props: { logs: Array<Event> }) {
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
