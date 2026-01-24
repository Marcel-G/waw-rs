import { initSync, __web_thread_worker_entry, Pointer, type Task, type Message } from '@shim.js'

onmessage = async event => {
	onmessage = null
	const [module, memory, stackSize, [workletLock, workerLock], task, message] = event.data as [
		WebAssembly.Module,
		WebAssembly.Memory,
		number | undefined,
		[number, number],
		Pointer<typeof Task>,
		Pointer<typeof Message>,
	]

	const memoryArray = new Int32Array(memory.buffer)

	Atomics.wait(memoryArray, workletLock, 1)
	Atomics.add(memoryArray, workerLock, 1)

	while (Atomics.load(memoryArray, workletLock) === 1) {
		if (Atomics.sub(memoryArray, workerLock, 1) === 1) Atomics.notify(memoryArray, workerLock)

		Atomics.wait(memoryArray, workletLock, 1)
		Atomics.add(memoryArray, workerLock, 1)
	}

	initSync({ module, memory, thread_stack_size: stackSize })

	if (Atomics.sub(memoryArray, workerLock, 1) === 1) Atomics.notify(memoryArray, workerLock)

	const terminateIndex = await __web_thread_worker_entry(task, message)
	Atomics.store(memoryArray, terminateIndex, 1)
	Atomics.notify(memoryArray, terminateIndex)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
