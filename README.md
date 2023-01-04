`waw-rs` should help you create WebAudio Worklets using Rust, without crying.

I would recommend to read [WebAssembly/Rust Tutorial: Pitch-perfect Audio Processing](https://www.toptal.com/webassembly/webassembly-rust-tutorial-web-audio)

This is very all very experimental.

## Features

- Define event types, message types and audio params all from Rust
- Works in Chrome, Firefox & Safari

## Usage

To use waw-rs in your project, add it as a dependency in your Cargo.toml:

```toml
waw = { git = "https://github.com/Marcel-G/waw-rs" }
```

To build the wasm module & JS bindings, you can use [wasm-pack](https://rustwasm.github.io/wasm-pack/)

You can generate a new RustWasm project using:

```bash
wasm-pack new <project_name>
```

Then, in your Rust code, simply implement the `AudioModule` trait and call the `waw::main!` macro on your struct:

`src/lib.rs`

```rust
use waw::{
  worklet::{ AudioModule, Emitter },
  buffer::{ AudioBuffer, ParamBuffer }
};

struct MyWorklet;

impl AudioModule for MyWorklet {
  fn create(_emitter: Emitter<Self::Event>) -> Self { MyWorklet }
  fn process(&mut self, audio: &mut AudioBuffer, params: &ParamBuffer<Self::Param>) {
    // Implement process
  }
}

waw::main!(MyWorklet);
```

Building the project with `wasm-pack` will generate [AudioWorkletProcessor](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor) and corresponding [AudioWorkletNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode).

```bash
wasm-pack build --target web
```

They can then be used from JavaScript:

```typescript
import init, { MyWorklet } from "./pkg/<project_name>";

const main = async () => {
  // Initialise the wasm module
  await init();

  // Create an audio context
  const context = new AudioContext();

  // Call install on the generated worklet node
  const worklet = await MyWorklet.install(context);

  // Connect the audio node to WebAudio graph
  worklet.node().connect(context.destination);

  // Wait for some interaction on the page before starting the audio
  const handle_interaction = () => {
    void context?.resume();
  };
  document.addEventListener("click", handle_interaction, { once: true });
};

main();
```

See the [demo project](https://github.com/Marcel-G/waw-rs/tree/main/demo) for a full example.

## Links

- [wasm-bindgen WASM audio worklet](https://rustwasm.github.io/wasm-bindgen/examples/wasm-audio-worklet.html#wasm-audio-worklet)
