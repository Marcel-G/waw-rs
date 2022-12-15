import init, { WorkletANode, WorkletBNode } from "./pkg/wasm_worklet_demo";

const main = async () => {
  await init();

  const context = new AudioContext();

  // Wait for some interaction on the page before starting the audio
  const handle_interaction = () => {
    void context?.resume()
  }
  document.addEventListener('click', handle_interaction, { once: true })

  // -- Initialisation Test --
  console.log('Worklet A loading...');
  const node_a = await WorkletANode.install(context);
  console.log('Worklet A done');

  console.log('Worklet B loading...');
  const node_b = await WorkletBNode.install(context);
  console.log('Worklet B done');

  console.log('Worklet B2 loading...');
  const node_b2 = await WorkletBNode.install(context);
  console.log('Worklet B2 done');

  // -- Audio Output Test --
  // Connect node to output
  node_a.node().connect(context.destination);
  node_b.node().connect(context.destination);
  node_b2.node().connect(context.destination);

  // -- Command Test --
  let count = 0;
  setInterval(() => {
    count++;
    node_a.command({ Count: count });
  }, 1000);

  // -- Subscribe Test --
  node_a.subscribe((event) => {
    console.log("Got some event", event)
  })

  // -- AudioParam Test --
  // Get the frequency AudioParam
  const frequency = node_a.get_param('Frequency');

  // Connect frequency AudioParam to the slider
  const frequencyControl = document.querySelector<HTMLInputElement>("#frequency");

  if (!frequencyControl) throw new Error("Can't find frequency slider")

  frequencyControl.addEventListener(
    "input",
    () => {
      frequency.value = parseInt(frequencyControl.value, 10);
    },
    false
  );
};

main();