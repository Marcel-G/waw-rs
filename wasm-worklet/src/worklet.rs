// This is a not-so-clean approach to get the current bindgen ES module URL
// in Rust. This will fail at run time on bindgen targets not using ES modules.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use js_sys::{global, Array, Reflect};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast, JsValue,
};
use web_sys::{
    AudioWorkletNodeOptions, AudioWorkletProcessor, Blob, BlobPropertyBag, MessageEvent, Url,
};

use crate::{
    buffer::Buffer,
    types::{AudioModule, AudioModuleDescriptor, InternalMessage, ParamMap, WorkletOptions},
    utils::{environment::assert_worklet, import_meta},
};

/// Returns a float that represents the sample rate of the associated BaseAudioContext.
pub fn sample_rate() -> f64 {
    Reflect::get(&global(), &"sampleRate".into())
        .unwrap()
        .as_f64()
        .unwrap()
}

/// Returns a double that represents the ever-increasing context time of the audio block being processed. It is equal to the currentTime property of the BaseAudioContext the worklet belongs to.
pub fn current_time() -> f64 {
    Reflect::get(&global(), &"currentTime".into())
        .unwrap()
        .as_f64()
        .unwrap()
}

/// Returns an integer that represents the ever-increasing current sample-frame of the audio block being processed. It is incremented by 128 (the size of a render quantum) after the processing of each audio block.
pub fn current_frame() -> usize {
    Reflect::get(&global(), &"currentFrame".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize
}

pub struct Processor<M: AudioModule> {
    rs_processor: Arc<Mutex<M>>,
    js_processor: AudioWorkletProcessor,
    enabled: Arc<AtomicBool>,
    input: Buffer<128>,
    output: Buffer<128>,
    params: ParamMap<M::Param>,
    message_callback: Option<Closure<dyn Fn(MessageEvent)>>,
}

impl<M: AudioModule + 'static> Processor<M> {
    pub fn new(rs_processor: M, js_processor: AudioWorkletProcessor) -> Self {
        assert_worklet();
        // Use the js options to to allocate the buffers
        // `options` is non-standard, it's manually attached to `AudioWorkletProcessor` in the constructor.
        let js_options: AudioWorkletNodeOptions = Reflect::get(&js_processor, &"options".into())
            .expect("Can't find options on AudioWorkletProcessor")
            .unchecked_into();

        let options = WorkletOptions::from(js_options);

        let input = Buffer::new(
            options.number_of_inputs.try_into().unwrap(),
            options.channel_count.try_into().unwrap(),
        );
        let output = Buffer::new(
            options.number_of_outputs.try_into().unwrap(),
            options.output_channel_count.try_into().unwrap(),
        );

        Processor {
            rs_processor: Arc::new(Mutex::new(rs_processor)),
            js_processor,
            enabled: Arc::new(AtomicBool::new(true)),
            input,
            output,
            params: Default::default(),
            message_callback: None,
        }
    }
    // Setup bi-directional messaging
    pub fn connect(&mut self) {
        // Add handler for inbound commands to module.
        let rs_processor = self.rs_processor.clone();
        let enabled = self.enabled.clone();
        let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(internal_message) =
                serde_wasm_bindgen::from_value::<InternalMessage>(event.data())
            {
                match internal_message {
                    InternalMessage::Destroy => {
                        enabled.store(false, Ordering::Relaxed);
                    }
                }
            } else {
                let message = serde_wasm_bindgen::from_value::<M::Command>(event.data()).unwrap(); // @todo message parsing handle error

                rs_processor.lock().unwrap().on_command(message);
            }
        }) as Box<dyn Fn(MessageEvent)>);

        self.js_processor
            .port()
            .unwrap()
            .add_event_listener_with_callback("message", callback.as_ref().unchecked_ref())
            .unwrap();

        self.message_callback = Some(callback);

        // Add handler for outbound events from module.
        let port = self.js_processor.port().unwrap();

        self.rs_processor
            .lock()
            .unwrap()
            .add_event_listener_with_callback(Box::new(move |event: M::Event| {
                let event = serde_wasm_bindgen::to_value(&event).expect("should work"); // @todo

                port.post_message(&event).expect("Failed to post message");
            }));

        self.js_processor.port().unwrap().start();
    }

    //  Each channel has 128 samples. Inputs[n][m][i] will access n-th input,
    //  m-th channel of that input, and i-th sample of that channel.
    //  https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/process
    //
    //  The number of inputs and thus the length of that array is fixed at the construction of the node (see AudioWorkletNode).
    //  If there is no active node connected to the n-th input of the node, inputs[n] will be an empty array (zero input channels available).
    //  The number of channels in each input may vary, depending on channelCount and channelCountMode properties.
    //
    pub fn process(&mut self, input: &Array, output: &Array, params: &JsValue) -> bool {
        self.input.copy_from(input);
        self.params.copy_from(params);

        self.rs_processor.lock().unwrap().process(
            self.input.get_ref(),
            self.output.get_mut(),
            &self.params,
        );

        self.output.copy_to(output);

        self.enabled.load(Ordering::Relaxed)
    }
}

fn js_source<M: AudioModuleDescriptor>() -> String {
    let processor_name = M::processor_name();
    let parameter_descriptors = M::parameter_descriptor_json();

    format!(
        "
    registerProcessor(
      \"{processor_name}\",
      class {processor_name} extends AudioWorkletProcessor {{
        static get parameterDescriptors() {{
          return {parameter_descriptors}
        }}
        constructor(options) {{
          super();
          this.options = options;
          const [wasm_src] = options.processorOptions || [];
          this.init(wasm_src)
        }}

        async init(wasm_src) {{
          if (wasm_src) {{
            globalThis._module = await WebAssembly.compile(wasm_src);
          }}

          if (globalThis._module) {{
            await init(globalThis._module);
          }} else {{
            throw new Error(\"Failed to initialize wasm module\")
          }}

          this.processor = new bindgen.{processor_name}(this);
      
          this.port.postMessage({{ method: 'send_wasm_program_done' }})

          this.processor.connect();
        }}

        process(inputs, outputs, parameters) {{
            if (this.processor && !this.processor.process(inputs, outputs, parameters)) {{
                this.processor.free();
                return false;
            }};
            return true
        }}
      }}
    );
  "
    )
}

// Creates a small JS module to connect the js worklet to the rust worklet.
// It is created at runtime and served via a Blob URL.
//
// Ideally this `.js` file would be generated at compile time however, there is currently no way
// to reference the wasm-bindgen `.js` file at build time.
// https://github.com/rustwasm/wasm-bindgen/pull/3032
pub fn register_processor<Module: AudioModuleDescriptor>() -> Result<String, JsValue> {
    // Get the URL of the current wasm module
    let url = import_meta::url_js();
    let js_source = js_source::<Module>();

    // Connect up the JS Processor to the Rust processor
    let worklet_processor_js = format!(
        "
      import init, * as bindgen from \"{url}\";

      {js_source}
    "
    );

    nop();

    // Create a Blob so that the browser can download the js code from a URL
    Url::create_object_url_with_blob(&Blob::new_with_str_sequence_and_options(
        &js_sys::Array::of1(&JsValue::from(worklet_processor_js)),
        BlobPropertyBag::new().type_("text/javascript"),
    )?)
}

// polyfill.js
//
// Firefox does not support ES6 module syntax in the worklets. Worklet code needs to be transpiled. https://bugzilla.mozilla.org/show_bug.cgi?id=1572644
// We polyfill `Worklet.prototype.addModule` to do this on-the-fly with esbuild if needed.
//
// TextEncoder and TextDecoder are not available in [AudioWorkletGlobalScope](https://searchfox.org/mozilla-central/source/dom/webidl/AudioWorkletGlobalScope.webidl),
// but there is a dirty workaround: install stub implementations of these classes in globalThis.
// https://github.com/rustwasm/wasm-bindgen/blob/main/examples/wasm-audio-worklet/src/polyfill.js
//
// In some cases (error logging, JSON::parse / JSON::stringify) a proper polyfill is necessary rather than just a stub.
//
// > Note about js-snppets https://rustwasm.github.io/wasm-bindgen/reference/js-snippets.html
//   Currently import statements are not supported in the JS file. This is a restriction we may lift in the future once we settle on a good way to support this.
//   For now, though, js snippets must be standalone modules and can't import from anything else.
#[wasm_bindgen(module = "/src/polyfill.js")]
extern "C" {
    fn nop();
}
