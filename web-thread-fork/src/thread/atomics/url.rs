//! Worker script.

use js_sys::Array;
use web_sys::{Blob, BlobPropertyBag, Url};

/// Wrapper around object URL to the worker script.
#[derive(Debug)]
pub(super) struct ScriptUrl(String);

impl Drop for ScriptUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).expect("`URL.revokeObjectURL()` should never throw");
	}
}

impl ScriptUrl {
	/// Creates a new [`Url`].
	pub(super) fn new(script: &str) -> Self {
		let sequence = Array::of1(&script.into());
		let property = BlobPropertyBag::new();
		property.set_type("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property)
			.expect("`new Blob()` should never throw");

		let url = Url::create_object_url_with_blob(&blob)
			.expect("`URL.createObjectURL()` should never throw");

		Self(url)
	}

	/// Returns the object URL.
	#[must_use]
	pub(super) fn as_raw(&self) -> &str {
		&self.0
	}
}
