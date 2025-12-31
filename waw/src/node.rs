use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use web_sys::AudioWorkletNode;

/// A wrapper around `AudioWorkletNode` that signals the processor to stop when dropped.
///
/// This ensures that when the node is dropped on the main thread, the processor running
/// in the AudioWorklet thread will stop processing on the next process call.
pub struct AudioWorkletNodeWrapper {
    node: AudioWorkletNode,
    is_active: Arc<AtomicBool>,
}

impl AudioWorkletNodeWrapper {
    /// Creates a new wrapper around an AudioWorkletNode with a shared active state.
    pub(crate) fn new(node: AudioWorkletNode, is_active: Arc<AtomicBool>) -> Self {
        Self { node, is_active }
    }

    /// Returns a reference to the underlying AudioWorkletNode.
    pub fn node(&self) -> &AudioWorkletNode {
        &self.node
    }

    /// Consumes the wrapper and returns the underlying AudioWorkletNode.
    ///
    /// Note: This will prevent the Drop implementation from running, so the processor
    /// will continue processing even after the wrapper is dropped.
    pub fn into_inner(self) -> AudioWorkletNode {
        let manual = ManuallyDrop::new(self);
        let node = manual.node.clone();
        manual.is_active.store(true, Ordering::Release);
        node
    }
}

impl Deref for AudioWorkletNodeWrapper {
    type Target = AudioWorkletNode;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl Drop for AudioWorkletNodeWrapper {
    fn drop(&mut self) {
        self.is_active.store(false, Ordering::Release);
    }
}

impl Clone for AudioWorkletNodeWrapper {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            is_active: self.is_active.clone(),
        }
    }
}
