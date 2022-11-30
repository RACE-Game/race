
/**
 * Race Client
 * A client in Race Protocol, it will handle all communication between transactors and emit events to DApp.
 *
 * Usage:
 *      const event_handler = console.log;
 *      let client = new RaceClient(event_handler);
 *      client.start();
 **/
export default class RaceClient {

  addr;
  eventHandler;

  constructor({eventHandler, addr}) {
    this.addr = addr;
    this.eventHandler = eventHandler;
  }

  async start() {
    console.log("Start race client");
    let module = await import("./pkg");
    this.registerEventHandler(this.eventHandler);
    await module.start(this.addr);
  }

  registerEventHandler(eventHandler) {
    window.addEventListener("message", (event) => {
      if (event.origin != "http://localhost:8000") return;
      if (event.data.target != "race-client") return;
      event_handler(event);
    })
  }
}

window.RaceClient = RaceClient;
