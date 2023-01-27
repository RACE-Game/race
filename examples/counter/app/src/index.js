let client = undefined;
let helper = undefined;
let events = [];
let profile = undefined;

async function onClickJoinButton() {
  await client.join(0, 100n);
}

async function onClickIncreamentButton() {
  await client.submit_event({ "Increase": 1 });
}

async function onClickRandomPokerButton() {
  await client.submit_event("RandomPoker");
}

async function onClickExitButton() {
  await client.exit();
}

async function onClickCreateProfile() {
  let nick = document.getElementById("input-nick").value;
  if (!nick || nick === "") {
    alert("Enter the nick name");
  }
  await helper.create_profile(nick, nick, "");
  profile = await helper.get_profile(nick);
  document.getElementById("player-nick").innerText = profile.nick;
}

function render(event, state) {
  document.getElementById("value").innerText = "" + state.value;
  document.getElementById("num_of_players").innerText = "" + state.num_of_players;
  document.getElementById("num_of_servers").innerText = "" + state.num_of_servers;
  document.getElementById("poker").innerText = "" + state.poker_card;
  console.log("New state =>", state);
  if (event !== null) {
    events.push(event);
    let innerHTML = "";
    for (let e of events) {
      innerHTML += "<p>" + JSON.stringify(e) + "</p>";
    }
    document.getElementById("events").innerHTML = innerHTML;
  }
}

function onStateUpdated(addr, context, state) {
  console.log("Updated context =>", context);
  render(context.event, state);
}

async function connect(addr) {
  console.log("Connect to game: %s", addr);
  const { AppClient } = await import("../../../../sdk/pkg");
  client = await AppClient.try_init(
    "facade", "ws://localhost:12002", profile.addr, addr, onStateUpdated
  );
  document.getElementById("join-btn").addEventListener("click", onClickJoinButton);
  document.getElementById("incr-btn").addEventListener("click", onClickIncreamentButton);
  document.getElementById("exit-btn").addEventListener("click", onClickExitButton);
  document.getElementById("poker-btn").addEventListener("click", onClickRandomPokerButton);
  client.attach_game();
  console.log("Game attached");
}


(async function() {
  const { AppHelper } = await import("../../../../sdk/pkg");
  helper = await AppHelper.try_init("facade", "ws://localhost:12002");
  document.getElementById("btn-create-profile").addEventListener("click", onClickCreateProfile);
  let games = await helper.list_games(["DEFAULT_REGISTRATION_ADDRESS"]);
  console.log("Fetch games from registrations =>", games);
  let container = document.getElementById("games");
  container.innerHTML = "";
  for (let game of games) {
    let item = document.createElement("div");
    item.style.padding = "1rem";
    let title = document.createElement("button");
    title.innerText = game.title;
    title.classList.add("btn-sm");
    title.addEventListener("click", () => {
      if (!profile) {
        alert("Create profile first");
        return;
      }
      connect(game.addr);
    });
    item.appendChild(title);
    container.appendChild(item);
  }
})();
