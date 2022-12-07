
window.start_app = async function() {
  const race = await import("./pkg");
  const client = await race.Client.init("facade-program-addr");
  const event = { Increase: 10 };
  await client.dispatch_custom_event(event);
  await client.dispatch_custom_event(event);
  console.log(client.get_json_state());
}
