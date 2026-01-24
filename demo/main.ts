import init, { registerContext, FilterNode, OscillatorNode } from './pkg/waw_demo';

const main = async () => {
  await init()
  const context = await registerContext();

  const osc_1 = new OscillatorNode(context, 110.0)
  const filter_1 = new FilterNode(context, 440.0)
  osc_1.node.connect(filter_1.node);
  filter_1.node.connect(context.destination);

  const frequency = osc_1.node.parameters.get('frequency')!

  const handle_interaction = async () => {
    void context?.resume();
  };
  

  const frequencyControl = document.querySelector<HTMLInputElement>("#frequency");
  if (!frequencyControl) throw new Error("Can't find frequency slider")
  frequencyControl.addEventListener(
    "input",
    () => {
      frequency.value = parseInt(frequencyControl.value, 10)
    },
    false
  );

  document.addEventListener("click", handle_interaction, { once: true });
};

main();
