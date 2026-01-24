import init, { registerContext, FilterNode, OscillatorNode } from './pkg/waw_demo';
import shimUrl from './pkg/waw_demo?url';

// Keep nodes in module scope to prevent garbage collection
let osc_1: OscillatorNode;
let filter_1: FilterNode;

const main = async () => {
  await init()
  const context = await registerContext(new URL(shimUrl, import.meta.url).href);

  osc_1 = new OscillatorNode(context, 110.0)
  filter_1 = new FilterNode(context, 440.0)
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

