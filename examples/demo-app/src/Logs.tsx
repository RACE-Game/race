import { useContext, useEffect, useState } from "react";
import GameContext from "./game-context";

function Logs() {
  const { context } = useContext(GameContext);
  const [events, setEvents] = useState<any[]>([]);

  useEffect(() => {
    if (context !== undefined) {
      if (context.event !== null) {
        console.log(context.event);
        setEvents(events => {
          events.push(context.event);
          return events;
        });
      }
    }
  }, [context]);

  return (
    <div className="h-full w-full relative">
      <div className="absolute p-4 rounded-lg border border-gray-500 inset-0 overflow-y-scroll">
        <h4 className="font-bold">Events:</h4>
        {
          events.map((e, i) => (
            <div key={i} className="p-1">
              {JSON.stringify(e)}
            </div>
          ))
        }
      </div>
    </div>
  );
}

export default Logs;
