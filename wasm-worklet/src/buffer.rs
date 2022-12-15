use js_sys::{Array, Float32Array};
use rsor::Slice;
use wasm_bindgen::JsCast;

pub struct Buffer<const N: usize> {
    buffer: Vec<Vec<[f32; N]>>,
    slice: Slice<[[f32; N]]>,
}

impl<const N: usize> Buffer<N> {
    pub fn new(n_inputs: usize, n_channels: usize) -> Self {
        let buffer = vec![vec![[0.0; N]; n_channels]; n_inputs];
        Buffer::<N> {
            buffer,
            slice: Slice::new(),
        }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut [&mut [[f32; N]]] {
        self.slice.from_muts(&mut self.buffer)
    }

    #[inline]
    pub fn get_ref(&mut self) -> &[&[[f32; N]]] {
        self.slice.from_refs(&self.buffer)
    }

    // Copy the contents of this JS typed array into the destination Rust slice.
    #[inline]
    pub fn copy_from(&mut self, input: &Array) {
        input
            .iter()
            .zip(self.get_mut())
            .for_each(|(input_js, input_rs)| {
                // Empty all the buffers
                input_rs
                    .iter_mut()
                    .for_each(|b| b.fill(0.0));

                let channels = input_js.unchecked_into::<Array>();
                channels
                    .iter()
                    .zip(input_rs.iter_mut())
                    .for_each(|(channel_js, channels_rs)| {
                        let channel = channel_js.unchecked_into::<Float32Array>();
                        channel.copy_to(channels_rs)
                    })
            })
    }

    // Copy the contents of the source Rust slice into this JS typed array.
    #[inline]
    pub fn copy_to(&mut self, input: &Array) {
        input
            .iter()
            .zip(self.get_ref())
            .for_each(|(input_js, input_rs)| {
                let channels = input_js.unchecked_into::<Array>();
                channels
                    .iter()
                    .zip(input_rs.iter())
                    .for_each(|(channel_js, channels_rs)| {
                        let channel = channel_js.unchecked_into::<Float32Array>();
                        channel.copy_from(channels_rs)
                    })
            })
    }
}
