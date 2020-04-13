//! Contains the Source trait that must be implemented by configuration sources.
use crate::item::StringItem;
use crate::confpath::ConfPath;

/// Trait that must be implemented by configuration sources.
pub trait Source {
	/// Get a configuration option.
	///
	/// This method is called by the configuration framework to retrieve the
	/// value for a configuration option. The configuration source must return
	/// `Some(StringItem)` with the `StringItem` containing the configuration
	/// value or None to signal that the item is not known to this configuration
	/// provider.
	///
	/// There is a special distinction between returning `None` and returning
	/// `Some(StringItem)` with an empty values vector.
	///
	/// - Returning `None` lets the configuration system continue the search
	///   for an other configuration provider that may know about this value.
	/// - Returning `Some(StringItem)` will stop the search for other
	///   configuration providers and let the empty value travel down the
	///   pipeline.
	///
	/// Eventually returning `None` and `Some(StringItem)` with an empty values
	/// vector yields the same results.
	///
	/// See [`Item`](../item/index.html) for more Information.
	fn get(&self, key: ConfPath) -> Option<StringItem>;
}
