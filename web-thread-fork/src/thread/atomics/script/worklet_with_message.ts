import {
	initSync,
	__web_thread_worklet_register,
	__web_thread_worklet_entry,
	Pointer,
	type Data,
	type Message,
	type Task,
} from '@shim.js'
import { __WebThreadProcessor, AudioParamDescriptor } from 'web_thread_worklet'

interface AudioWorkletProcessorExt extends AudioWorkletProcessor {
	__web_thread_this: __WebThreadProcessor
	continueProcessing: boolean
}

globalThis.__web_thread_register_processor = (name, processor) => {
	globalThis.registerProcessor(
		name,
		class extends AudioWorkletProcessor implements AudioWorkletProcessorImpl {
			constructor(options: AudioWorkletNodeOptions) {
				super()
				const this_ = this as AudioWorkletProcessor as AudioWorkletProcessorExt
				this_.__web_thread_this = processor.instantiate(this, options)
			}

			process(
				this: AudioWorkletProcessor,
				inputs: Float32Array[][],
				outputs: Float32Array[][],
				parameters: Record<string, Float32Array>
			): boolean {
				const this_ = this as AudioWorkletProcessorExt
				return this_.__web_thread_this.process(inputs, outputs, parameters)
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
			const this_ = this as AudioWorkletProcessor as AudioWorkletProcessorExt

			const [module, memory, stackSize, workletLock, data] = options.processorOptions as [
				WebAssembly.Module,
				WebAssembly.Memory,
				number | undefined,
				number,
				Pointer<typeof Data>,
			]

			initSync({ module, memory, thread_stack_size: stackSize })
			const memoryArray = new Int32Array(memory.buffer)
			Atomics.store(memoryArray, workletLock, 0)
			Atomics.notify(memoryArray, workletLock)

			__web_thread_worklet_register(data)

			this_.continueProcessing = true
			this_.port.onmessage = event => {
				this_.continueProcessing = false
				this_.port.onmessage = null
				const [task, message] = event.data as [
					Pointer<typeof Task> | undefined,
					Pointer<typeof Message>,
				]

				if (task === undefined) return

				__web_thread_worklet_entry(task, message, event.ports[0])
			}
		}

		process(this: AudioWorkletProcessorExt): boolean {
			return this.continueProcessing
		}
	}
)
