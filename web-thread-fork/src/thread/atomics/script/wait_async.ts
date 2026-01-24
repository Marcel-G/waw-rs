onmessage = event => {
	const [memory, index, value] = event.data as [WebAssembly.Memory, number, number]
	Atomics.wait(new Int32Array(memory.buffer), index, value)
	postMessage(undefined)
}
