const delay = ms => new Promise(res => setTimeout(res, ms));

async function main() {
  const { WasmAppClient } = await import("../../../../client/pkg");
  const appClient = new WasmAppClient('facade', 'ws://localhost:12002', 'facade-game-addr');
  await appClient.initialize();
}

main();
