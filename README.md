# waw-rs

`waw-rs` helps you create Web Audio Worklets using Rust, without crying.

See [WebAssembly/Rust Tutorial: Pitch-perfect Audio Processing](https://www.toptal.com/webassembly/webassembly-rust-tutorial-web-audio)

This is all very experimental.

## Usage

Add waw-rs to your Cargo.toml:

```toml
waw = { git = "https://github.com/Marcel-G/waw-rs" }
```

Implement the `Processor` trait and register your audio node:

```rust
use wasm_bindgen::prelude::*;
use waw::{create_node, register, ParameterValues, Processor};

#[derive(Clone)]
pub struct MyData {
    pub frequency: f32,
}

pub struct MyProcessor {
    phase: f32,
    frequency: f32,
}

impl Processor for MyProcessor {
    type Data = MyData;

    fn new(data: Self::Data) -> Self {
        Self { phase: 0.0, frequency: data.frequency }
    }

    fn process(
        &mut self,
        _inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValues,
    ) {
        // ... your audio processing logic
    }
}

#[wasm_bindgen]
pub struct MyNode {
    node: web_sys::AudioWorkletNode,
}

#[wasm_bindgen]
impl MyNode {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: &web_sys::AudioContext, frequency: f32) -> Result<MyNode, JsValue> {
        let data = MyData { frequency };
        let node = create_node::<MyProcessor>(ctx, "my-processor", data)?;
        Ok(MyNode { node })
    }

    #[wasm_bindgen(getter)]
    pub fn node(&self) -> web_sys::AudioWorkletNode {
        self.node.clone()
    }
}

register!(MyProcessor, "my-processor");
```

Build with wasm-pack:

```bash
wasm-pack build --target web
```

Use in JavaScript:

```typescript
import init, { MyNode, registerContext } from './pkg/your_project';

const main = async () => {
  await init();
  const context = await registerContext();

  const node = new MyNode(context, 440.0);
  node.node.connect(context.destination);

  document.addEventListener('click', () => context.resume(), { once: true });
};

main();
```

See the [demo](demo) for a complete example.

## Links

- [wasm-bindgen WASM audio worklet](https://rustwasm.github.io/wasm-bindgen/examples/wasm-audio-worklet.html#wasm-audio-worklet)
