//! Source supplying static defaults
//! 
//! The 'Defaults' source supplies static values for configuration items. It can
//! be used to add static configuration values as defaults or to overwrite
//! values originating from a configuration file with values supplied at the
//! command line.
//!
//! Multiple `Defaults` sources can be added to the same `Config` instance. The
//! sources are queried from first to last. Depending on the order the
//! `Defaults` source can supply defaults or overwrite values.
//! 
//! ```text
//! +------------+---------------------+
//! | Defaults   | from command line   |
//! +------------+---------------------+
//! | ConfigText | to read config file |
//! +------------+---------------------+
//! | Defaults   | as defaults         |
//! +------------+---------------------+
//! ```
//! 
//! ## Example
//! 
//! ```rust
//! use justconfig::Config;
//! use justconfig::ConfPath;
//! use std::ffi::OsStr;
//! use justconfig::item::ValueExtractor;
//! use justconfig::sources::defaults::Defaults;
//! use justconfig::sources::env::Env;
//!
//! let mut conf = Config::default();
//! let mut defaults = Defaults::default();
//! let mut env = Env::new(&[(ConfPath::from(&["Workdir"]), OsStr::new("WORKDIR"))]);
//! 
//! defaults.set(ConfPath::from(&["Workdir"]), "/tmp", "Default Workdir /tmp");
//! 
//! conf.add_source(defaults);
//! conf.add_source(env);
//!
//! // If the environment variabel `WORKDIR` ist not set use `/tmp´ as a defualt.
//! let path: String = conf.get(ConfPath::from(&["Workdir"])).value().unwrap();
//! assert_eq!(path, "/tmp");
//! ```
use crate::source::Source;
use crate::item::{SourceLocation, StringItem, Value};
use crate::confpath::ConfPath;
use std::rc::Rc;
use std::collections::HashMap;
use std::fmt;

/// Source location for the Defaults configuration source.
/// 
/// This value is used to store the source of every configuration value for
/// use in error messages.
#[derive(Debug)]
pub struct DefaultSourceLocation {
	source: String
}

impl DefaultSourceLocation {
	fn new(source: &str) -> Rc<Self> {
		Rc::new(Self {
			source: source.to_owned()
		})
	}
}

impl fmt::Display for DefaultSourceLocation {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "default from {}", self.source)
	}
}

impl SourceLocation for DefaultSourceLocation {}

/// Implements the Defaults source.
pub struct Defaults {
	items: HashMap<ConfPath, StringItem>
}

impl Defaults {
	/// Creates a new defaults source.
	///
	/// The created `Defaults` instance does not contain any values.
	///
	/// See the [`defaults`](index.html) module for more information.
	pub fn default() -> Box<Self> {
		Box::new(Self {
			items: HashMap::default()
		})
	}

	/// Returns a `StringItem` instance that can be used to manipulate the
	/// values for the item referenced by the key. If there is no `StringItem`
	/// instance available for this key a new one is created.
	fn get_item(&mut self, key: ConfPath) -> &mut StringItem {
		self.items.entry(key.clone()).or_insert_with(|| StringItem::new(key))
	}

	/// Clear all values for the given key.
	pub fn empty(&mut self, key: ConfPath) {
		self.get_item(key).clear();
	}

	/// Set the value of this key
	/// 
	/// Sets the value of the given `key` to the passed `value`. All previously
	/// set values are discarded.
	/// 
	/// The `source` parameter specifies a string that is used to identify the
	/// source for this configuration information in error messages.
	/// 
	/// See [`put`](#method.put) for an example.
	pub fn set(&mut self, key: ConfPath, value: &str, source: &str) {
		self.get_item(key).clear().push(Value::new(value.to_owned(), DefaultSourceLocation::new(source)));
	}

	/// Add a value to the configuration values of this key
	/// 
	/// Adds a `value` to the configuration values of the given `key`. This can
	/// be used to add multiple values for a configuration item.
	/// 
	/// If you want to clear all previously set values instead of adding the
	/// value to the list of configuration values use [`set`](#method.set).
	/// 
	/// The `source` parameter specifies a string that is used to identify the
	/// source for this configuration information in error messages.
	/// 
	/// ## Example
	/// 
	/// 
	/// ```rust
	/// use justconfig::Config;
	/// use justconfig::ConfPath;
	/// use justconfig::item::ValueExtractor;
	/// use justconfig::sources::defaults::Defaults;
	///
	/// let mut conf = Config::default();
	/// let mut defaults = Defaults::default();
	/// 
	/// defaults.set(ConfPath::from(&["Destination"]), "/tmp", "Default destination directory");
	/// defaults.set(ConfPath::from(&["Sources"]), "/srv/source/a", "Default source directory A");
	/// defaults.put(ConfPath::from(&["Sources"]), "/srv/source/b", "Default source directory B");
	/// 
	/// conf.add_source(defaults);
	///
	/// let destination: String = conf.get(ConfPath::from(&["Destination"])).value().unwrap();
	/// assert_eq!(destination, "/tmp");
	/// 
	/// let sources: Vec<String> = conf.get(ConfPath::from(&["Sources"])).values().unwrap();
	/// assert_eq!(sources, ["/srv/source/a", "/srv/source/b"]);
	/// ```
	pub fn put(&mut self, key: ConfPath, value: &str, source: &str) {
		self.get_item(key).push(Value::new(value.to_owned(), DefaultSourceLocation::new(source)));
	}
}

impl Source for Defaults {
	fn get(&self, key: ConfPath) -> Option<StringItem> {
		self.items.get(&key).cloned()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Config;
	use crate::ConfPath;
	use crate::error::ConfigError;
	use crate::item::ValueExtractor;

	#[test]
	fn defaults() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		// Simply setting a value
		d.set(ConfPath::from(&["testA"]), "AaA", "sourceA");

		// Setting and putting to have multiple values
		d.set(ConfPath::from(&["testB"]), "BbB", "sourceB.1");
		d.put(ConfPath::from(&["testB"]), "bBb", "sourceB.2");

		// Empty to clear everything already set
		d.set(ConfPath::from(&["testC"]), "cCc", "sourceC.1");
		d.empty(ConfPath::from(&["testC"]));
		d.put(ConfPath::from(&["testC"]), "CcC", "sourceC.2");

		// Setting clears all previous values
		d.set(ConfPath::from(&["testD"]), "dDd", "sourceD.1");
		d.put(ConfPath::from(&["testD"]), "ddD", "sourceD.2");
		d.set(ConfPath::from(&["testD"]), "DdD", "sourceD.3");

		// First put is like set
		d.put(ConfPath::from(&["testE"]), "EeE", "sourceE");

		c.add_source(d);

		assert_eq!((c.get(ConfPath::from(&["testA"])).value() as Result<String, ConfigError>).unwrap(), "AaA");
		assert_eq!((c.get(ConfPath::from(&["testB"])).values() as Result<Vec<String>, ConfigError>).unwrap(), ["BbB", "bBb"]);
		assert_eq!((c.get(ConfPath::from(&["testC"])).value() as Result<String, ConfigError>).unwrap(), "CcC");
		assert_eq!((c.get(ConfPath::from(&["testD"])).value() as Result<String, ConfigError>).unwrap(), "DdD");
		assert_eq!((c.get(ConfPath::from(&["testE"])).value() as Result<String, ConfigError>).unwrap(), "EeE");
	}
}