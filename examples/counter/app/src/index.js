const delay = ms => new Promise(res => setTimeout(res, ms));

async function main() {
  // const { WasmAppClient } = await import("../../../../client/pkg");
  // const appClient = new WasmAppClient('facade', 'ws://localhost:12002', 'facade-game-addr');
  // await appClient.initialize();

  const { AppHelper } = await import("../../../../client/pkg");
  const helper = new AppHelper('facade', 'ws://localhost:12002');
  await helper.initialize();
  await delay(5000);
  console.log("helper initalized");
  console.log("helper initalized");
  console.log("helper initalized");
  console.log("helper initalized");
  await helper.create_game('game-bundle-addr', 2);
}

main();
