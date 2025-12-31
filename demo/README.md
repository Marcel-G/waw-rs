# waw-rs demo

Welcome to the waw-rs demo project! This project demonstrate how to create a super simple WebAudio application in the browser using vite and waw-rs.

## Requirements

This project requires WebAssembly target features `+atomics` and `+bulk-memory`. These are already configured in the workspace's `.cargo/config.toml` file at the repository root.

## Running the project

To install wasm-pack:
```bash
cargo install wasm-pack
```

To install npm dependencies:
```bash
npm install
```

To build the project:
```bash
npm run build
```

To start the development server and open the browser:
```bash
npm run dev -- --open
```
