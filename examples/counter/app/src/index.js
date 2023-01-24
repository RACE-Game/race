let client = undefined;
let events = [];

async function onClickJoinButton() {
  await client.join(0, 100n);
}

async function onClickIncreamentButton() {
  await client.submit_event({ "Increase": 1 });
}

async function onClickExitButton() {
  await client.exit();
}

function render(event, state) {
  document.getElementById("value").innerText = "" + state.value;
  document.getElementById("num_of_players").innerText = "" + state.num_of_players;
  document.getElementById("num_of_servers").innerText = "" + state.num_of_servers;
  if (event !== null) {
    events.push(event);
    let innerHTML = '';
    for (let e of events) {
      innerHTML += "<p>" + JSON.stringify(e) + "</p>";
    }
    document.getElementById("events").innerHTML = innerHTML;
  }
}

function onInited(_gameAddr, state) {
  render(null, state);
}

function onStateUpdated(_gameAddr, event, state) {
  render(event, state);
}

(async function() {
  const { AppClient } = await import("../../../../client/pkg");
  client = await AppClient.try_init('facade', 'ws://localhost:12002', 'Alice', 'COUNTER_GAME_ADDRESS', onInited, onStateUpdated);
  document.getElementById("join-btn").addEventListener("click", onClickJoinButton);
  document.getElementById("incr-btn").addEventListener("click", onClickIncreamentButton);
  document.getElementById("exit-btn").addEventListener("click", onClickExitButton);
  client.attach_game();
  console.log("Game attached");
})();
