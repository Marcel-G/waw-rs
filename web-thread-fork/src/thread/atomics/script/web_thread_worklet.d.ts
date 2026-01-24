declare global {
	function __web_thread_register_processor(
		name: string,
		processor: __WebThreadProcessorConstructor
	): void
}

export class __WebThreadProcessor {
	process(
		inputs: Float32Array[][],
		outputs: Float32Array[][],
		parameters: Record<string, Float32Array>
	): boolean
}

export class __WebThreadProcessorConstructor {
	instantiate(
		this_: AudioWorkletProcessor,
		options: AudioWorkletNodeOptions
	): __WebThreadProcessor
	parameterDescriptors(): AudioParamDescriptor[]
}

// missing from `@types/audioworklet`
export interface AudioParamDescriptor {
	name: string
	automationRate: AutomationRate
	minValue: number
	maxValue: number
	defaultValue: number
}
