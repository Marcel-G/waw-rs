//! Async channel.

use std::fmt::{self, Debug, Formatter};
use std::future;
use std::sync::mpsc::{self, RecvError, SendError, TryRecvError};
use std::sync::Arc;
use std::task::Poll;

use atomic_waker::AtomicWaker;

/// Async version of [`std::sync::mpsc`].
pub(super) fn channel<T>() -> (Sender<T>, Receiver<T>) {
	let (sender, receiver) = mpsc::channel();
	let waker = Arc::new(AtomicWaker::new());

	let sender = Sender {
		inner: Some(Arc::new(sender)),
		waker: Arc::clone(&waker),
	};
	let receiver = Receiver { receiver, waker };

	(sender, receiver)
}

/// Async version of [`mpsc::Sender`].
pub(super) struct Sender<T> {
	/// Actual [`mpsc::Sender`].
	inner: Option<Arc<mpsc::Sender<T>>>,
	/// Shared [`Waker`](std::task::Waker) between [`Sender`] and [`Receiver`].
	waker: Arc<AtomicWaker>,
}

impl<T> Sender<T> {
	/// Send an `event` to the corresponding [`Receiver`].
	pub(super) fn send(&self, event: T) -> Result<(), SendError<T>> {
		self.inner
			.as_ref()
			.expect("`inner` not found")
			.send(event)?;
		self.waker.wake();

		Ok(())
	}
}

impl<T> Clone for Sender<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			waker: Arc::clone(&self.waker),
		}
	}
}

impl<T> Drop for Sender<T> {
	fn drop(&mut self) {
		if Arc::into_inner(self.inner.take().expect("`inner` not found")).is_some() {
			// At this point it is guaranteed that the last `Sender` has been dropped and
			// therefor `Receiver` will always return `TryRecvError::Disconnected`.
			self.waker.wake();
		}
	}
}

/// Async version of [`mpsc::Receiver`].
pub(super) struct Receiver<T> {
	/// Actual [`mpsc::Sender`].
	receiver: mpsc::Receiver<T>,
	/// Shared [`Waker`](std::task::Waker) between [`Receiver`] and [`Sender`].
	waker: Arc<AtomicWaker>,
}

impl<T> Debug for Receiver<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Receiver")
			.field("receiver", &self.receiver)
			.field("waker", &self.waker)
			.finish()
	}
}

impl<T> Receiver<T> {
	/// Attempts to return a pending value on this receiver without blocking.
	#[cfg(feature = "message")]
	pub(super) fn try_recv(&self) -> Result<T, TryRecvError> {
		self.receiver.try_recv()
	}

	/// Wait for the next event sent by the [`Sender`].
	pub(super) async fn next(&self) -> Result<T, RecvError> {
		future::poll_fn(|cx| match self.receiver.try_recv() {
			Ok(event) => Poll::Ready(Ok(event)),
			Err(TryRecvError::Empty) => {
				self.waker.register(cx.waker());

				match self.receiver.try_recv() {
					Ok(event) => Poll::Ready(Ok(event)),
					Err(TryRecvError::Empty) => Poll::Pending,
					Err(TryRecvError::Disconnected) => Poll::Ready(Err(RecvError)),
				}
			}
			Err(TryRecvError::Disconnected) => Poll::Ready(Err(RecvError)),
		})
		.await
	}
}
