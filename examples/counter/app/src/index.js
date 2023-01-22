let client = undefined;

async function onClickJoinButton() {
  await client.join(0, 100n);
}

async function onClickIncreamentButton() {
  await client.submit_event({ "Increase": 1 });
}

async function onClickExitButton() {
  await client.exit();
}

function onStateUpdated(_gameAddr, event, state) {
  console.log(event);
  console.log(state);
  const value = state.value;
  document.getElementById("value").innerText = "" + value;
}

(async function() {
  const { AppClient } = await import("../../../../client/pkg");
  client = await AppClient.try_init('facade', 'ws://localhost:12002', 'Alice', 'COUNTER_GAME_ADDRESS');
  document.getElementById("join-btn").addEventListener("click", onClickJoinButton);
  document.getElementById("incr-btn").addEventListener("click", onClickIncreamentButton);
  document.getElementById("exit-btn").addEventListener("click", onClickExitButton);
  client.attach_game_with_callback(onStateUpdated);
  console.log("Game attached");
})();
