import { initSync, __web_thread_worker_entry, Pointer, type Task, type Message } from '@shim.js'

onmessage = async event => {
	onmessage = null
	const [module, memory, stackSize, task, message] = event.data as [
		WebAssembly.Module,
		WebAssembly.Memory,
		number | undefined,
		Pointer<typeof Task>,
		Pointer<typeof Message>,
	]

	initSync({ module, memory, thread_stack_size: stackSize })
	const terminateIndex = await __web_thread_worker_entry(task, message)
	const memoryArray = new Int32Array(memory.buffer)
	Atomics.store(memoryArray, terminateIndex, 1)
	Atomics.notify(memoryArray, terminateIndex)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
