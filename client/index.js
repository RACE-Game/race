window.start_app = async function() {
  const module = await import("./pkg");
  const client = await module.RaceClient.init("facade-program-addr");
  const event = { Increase: 10 };
  await client.dispatch_custom_event(event);
  await client.dispatch_custom_event(event);
  console.log(client.get_json_state());
}
