// eslint-disable-next-line @typescript-eslint/no-unused-vars
type Pointer<T> = symbol
declare const Data: unique symbol
declare const Message: unique symbol
declare const Task: unique symbol

export function initSync(options: initSyncOptions): unknown

export interface initSyncOptions {
	module: BufferSource | WebAssembly.Module
	memory?: WebAssembly.Memory
	thread_stack_size?: number | undefined
}

export function __web_thread_worklet_entry(
	task: Pointer<typeof Task>,
	message?: Pointer<typeof Message>,
	port?: MessagePort
): void

export function __web_thread_worklet_register(data: Pointer<typeof Data>): void

export function __web_thread_worker_entry(
	task: Pointer<typeof Task>,
	message: Pointer<typeof Message>
): Promise<number>
