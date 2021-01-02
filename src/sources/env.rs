//! Environment source
//!
//! The environment source supplies environment variables to the configuration
//! system. By being its own source it can be ordered before or after other
//! sources. This way environment variables can override configuration settings
//! or can be used as a fallback.
//!
//! The environment source uses a mapping to translate the names of environment
//! variables into configuration paths. This mapping is passed to the
//! [`new`](Env::new) method. Environment variables not present within this
//! mapping are inaccessible by the configuration system. Adding a mapping for
//! an environment variable does *not* make sure it really exists.
//! 
//! ## Example
//! 
//! ```rust
//! use justconfig::Config;
//! use justconfig::ConfPath;
//! use std::ffi::OsStr;
//! use justconfig::item::ValueExtractor;
//! use justconfig::sources::env::Env;
//!
//! let mut conf = Config::default();
//!
//! conf.add_source(Env::new(&[
//!   (ConfPath::from(&["Path"]), OsStr::new("PATH")),
//!   (ConfPath::from(&["HomeDir"]), OsStr::new("HOME"))
//! ]));
//!
//! // Read the path from the environment
//! let path: String = conf.get(ConfPath::from(&["Path"])).value().unwrap();
//! ```
use crate::source::Source;
use crate::item::{SourceLocation, StringItem, Value};
use crate::confpath::ConfPath;
use std::ffi::{OsStr, OsString};
use std::collections::hash_map::HashMap;
use std::fmt;
use std::env;
use std::rc::Rc;

/// Source location for the Env configuration source.
/// 
/// This value is used to store the source of every configuration value for
/// use in error messages.
#[derive(Debug)]
struct EnvSourceLocation {
	env_name: OsString
}

impl EnvSourceLocation {
	pub fn new(env_name: &OsStr) -> Rc<Self> {
		Rc::new(Self {
			env_name: env_name.to_owned()
		})
	}
}

impl fmt::Display for EnvSourceLocation {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "env:{}", self.env_name.to_string_lossy())
	}
}

impl SourceLocation for EnvSourceLocation {}

/// Implements the environment source.
pub struct Env {
	env_mapping: HashMap<ConfPath, OsString>
}

impl Env {
	/// Creates a new environment source.
	///
	/// For creating the environment source a mapping between the configuration
	/// key and the name of the environment variable has to be created. This is done
	/// by passing a slice of tuples. The first element of the tuple defines the
	/// configuration path of the environment value and the second element defines
	/// the name of the environment variable.
	///
	/// See the [`env`](mod@env) module for more information.
	pub fn new(env_mapping: &[(ConfPath, &OsStr)]) -> Box<Self> {
		Box::new(Self {
			env_mapping: env_mapping.iter().map(|m| (m.0.clone(), m.1.to_owned())).collect()
		})
	}
}

impl Source for Env {
	fn get(&self, key: ConfPath) -> Option<StringItem> {
		if let Some(env_name) = self.env_mapping.get(&key) {
			env::var(env_name).ok().map(|v| StringItem::from(key, &[Value::new(v, EnvSourceLocation::new(env_name))]))
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Config;
	use crate::error::ConfigError;
	use crate::item::ValueExtractor;
	use std::env;

	fn prepare_test_config() -> Config{
		env::set_var(OsStr::new("existing_value"), OsStr::new("existing_value"));

		let mut c = Config::default();

		c.add_source(Env::new(&[
			(ConfPath::from(&["testA"]), OsStr::new("existing_value")),
			(ConfPath::from(&["testB"]), OsStr::new("non_existant_value"))
		]));

		c
	}

	#[test]
	fn existing_value() {
		let c = prepare_test_config();
		assert_eq!((c.get(ConfPath::from(&["testA"])).value() as Result<String, ConfigError>).unwrap(), "existing_value");
	}

	#[test]
	#[should_panic(expected = "ValueNotFound")]
	fn non_existant_env_name() {
		let c = prepare_test_config();

		(c.get(ConfPath::from(&["testB"])).value() as Result<String, ConfigError>).unwrap();
	}

	#[test]
	#[should_panic(expected = "ValueNotFound")]
	fn non_existant_value() {
		let c = prepare_test_config();

		(c.get(ConfPath::from(&["testC"])).value() as Result<String, ConfigError>).unwrap();
	}
}