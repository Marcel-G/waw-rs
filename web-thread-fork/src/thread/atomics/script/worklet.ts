import { initSync, __web_thread_worklet_entry, Pointer, type Task } from '@shim.js'
import {
	__WebThreadProcessorConstructor,
	__WebThreadProcessor,
	AudioParamDescriptor,
} from 'web_thread_worklet'

interface AudioWorkletProcessorExt extends AudioWorkletProcessor {
	__web_thread_this: __WebThreadProcessor
}

globalThis.__web_thread_register_processor = (
	name: string,
	processor: __WebThreadProcessorConstructor
) => {
	globalThis.registerProcessor(
		name,
		class extends AudioWorkletProcessor implements AudioWorkletProcessorImpl {
			constructor(options: AudioWorkletNodeOptions) {
				super()
				const this_ = this as AudioWorkletProcessor as AudioWorkletProcessorExt
				this_.__web_thread_this = processor.instantiate(this, options)
			}

			process(
				this: AudioWorkletProcessorExt,
				inputs: Float32Array[][],
				outputs: Float32Array[][],
				parameters: Record<string, Float32Array>
			): boolean {
				return this.__web_thread_this.process(inputs, outputs, parameters)
			}

			static get parameterDescriptors(): AudioParamDescriptor[] {
				return processor.parameterDescriptors()
			}
		}
	)
}

registerProcessor(
	'__web_thread_worklet',
	class extends AudioWorkletProcessor implements AudioWorkletProcessorImpl {
		constructor(options: AudioWorkletNodeOptions) {
			super()

			const [module, memory, stackSize, workletLock, task] = options.processorOptions as [
				WebAssembly.Module,
				WebAssembly.Memory,
				number | undefined,
				number,
				Pointer<typeof Task>,
			]

			initSync({ module: module, memory: memory, thread_stack_size: stackSize })
			const memoryArray = new Int32Array(memory.buffer)
			Atomics.store(memoryArray, workletLock, 0)
			Atomics.notify(memoryArray, workletLock)

			__web_thread_worklet_entry(task)
		}

		process(): boolean {
			return false
		}
	}
)
