# PROBLEM

When importing `pkg/waw_demo.js` into the worklet global scope module 

1. init the wasm module on the main thread
2. call into an `install()` fn
3. download the wasm file `pkg/waw_demo_bg.wasm`
4. generate the worklet module
		Which generates the URL to the js file file where init came from `pkg/waw_demo.js`

The problem is, the **bundler** puts everything into index.82aba589.js
which is a single bundle for the main thread
it does not have the same interface / exports / name as pkg/waw_demo.js 

> Uncaught (in promise) SyntaxError: The requested module 'http://localhost:4173/assets/index.82aba589.js' does not provide an export named 'default'

// bootstrapping wasm in the webaudio worklet

- Remove the need for js in the worklet
	cannot extend classes


https://www.toptal.com/webassembly/webassembly-rust-tutorial-web-audio

Recreate this demo?


OK - idea to try (that depends on vite) or some bundler that knows about .worklet / ?worker&url etc

1. generate a single JS file that registers all the worklets (in to separate files?)
2. generate an index file (blah.js) to collect them all..............
3. import worklet from 'blah.js?worker&url'
4. register all the modules at the same time with addModule(worklet)

OK what about without a bundler / agnostic

well It would be awesome if waw_demo.js was bundled but kept intact as a module such that the exported interface remains the same

preserve module interface
maybe prebundle the JS file?
`build.rs`?


---------------------

Hide away all of the BS inside the WASM module
- Find a way to load JS files as string into the wasm
- They can be executed using worklet.add_module() in the audio worklet thread.
	- TextEncoderDecoder polyfill
	- Import the js file (has to be with a dynamic import from JS to create a separate chunk)
	- Need to fix


What about building two