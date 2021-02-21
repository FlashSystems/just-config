//! The just-config crate supplies a methods for reading, transforming and
//! validating configuration information from multiple sources.
//!
//! Before you read any further let's answer a simple question: Is this
//! configuration library for you?
//!
//! * If you want to read a configuration file created by your user: yes
//! * If you want to flexibly process the information read from a configuration
//!   file: yes
//! * If you want to add environment variables or constants (command line
//!   parameters) to the mix: yes
//! * If you want an easy way to merge multiple ocnfiguration sources: yes
//! * If you want to add defaults or environment variables: yes
//! * If you want to read _and_ write a configuration file: no
//! * If you want your configuration file to specify data types: no
//! * If you want your configuration file to have significant whitespace or be a
//!   mess of brackets: no
//!
//! ## Navigating configuration information
//!
//! ## Configuration paths
//!
//! Configuration information in just-config is represented as a set of nodes.
//! Each node can have sub nodes forming a configuration tree. To not limit the
//! developer to a specific choice of separator character the node tree is
//! represented as an instance of the [ConfPath] struct.
//!
//! ## Configuration pipeline
//!
//! Configuration information in `just-config` is processed using a
//! configuration pipeline. You retrieve a configuration value from the
//! configuration source by calling [`get`](Config::get). Then the
//! configuration information is passed though
//! [processors](processors) and [validators](validators).
//!
//! ### Configuration source
//!
//! A configuration source is any struct that implements the
//! [`Source`] trait. This make the configuration
//! system very flexible. You can read configuration information from text
//! files, the network or from environment variables. All these configuration
//! sources can be mixed and matched.
//!
//! There are some [configuration sources already included](sources)
//! in just-config.
//!
//! Multiple configuration sources can be registered. They are tried in order of
//! their registration. The first configuration source that returns a value for
//! a configuration key is used. That way configuration sources can be layered.
//! See [`add_source`](Config::add_source) for more
//! information and an example.
//!
//! ### Processors
//!
//! The processors allow you to pre-process the value read from the
//! configuration source. Processors always operate on the string representation
//! of the value. Their purpose is to transform the string (trim, unquote,
//! unescape, etc.) and prepare it for conversion into the target data type.
//!
//! ### Validators
//!
//! As soon as the first validator is called the string value is converted into
//! the target data type. All validation takes place after the conversion. That
//! way the properties of the target data type can be used to validate the
//! information. The first validation step is always present, its the call to
//! the [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) method
//! of the target date type. If this conversion fails, the configuration value
//! is returned as invalid.
//!
//! As you might have guessed, all types implementing the `FromStr` trait are
//! able to be used as target data types for the configuration. In most cases
//! type inference does a very good job to provide the necessary type
//! information for `just-config`. Simply assign the configuration value to the
//! target data type and it will use the `FromStr` trait to convert the string
//! value from the configuration source into that type.
//!
//! # Examples
//!
//! ## Basic example
//!
//! ```no_run
//! use justconfig::Config;
//! use justconfig::ConfPath;
//! use justconfig::sources::text::ConfigText;
//! use justconfig::sources::env::Env;
//! use justconfig::sources::defaults::Defaults;
//! use justconfig::processors::Explode;
//! use justconfig::validators::Range;
//! use justconfig::item::ValueExtractor;
//! use std::ffi::OsStr;
//! use std::fs::File;
//!
//! let mut conf = Config::default();
//!
//! // Allow some environment variables to override configuration values read
//! // from the configuration file.
//! let config_env = Env::new(&[
//!   (ConfPath::from(&["searchPath"]), OsStr::new("SEARCH_PATH")),
//! ]);
//! conf.add_source(config_env);
//!
//! // Open the configuration file
//! let config_file = File::open("myconfig.conf").expect("Could not open config file.");
//! conf.add_source(ConfigText::new(config_file, "myconfig.conf").expect("Loading configuration file failed."));
//!
//! // Read the value `num_frobs` from the configuration file.
//! // Do not allow to use more than 10 frobs.
//! let num_frobs: i32 = conf.get(conf.root().push("num_frobs")).max(10).value()?;
//!
//! // Read a list of tags from the configuration file.
//! let tag_list: Vec<String> = conf.get(conf.root().push("tags")).values(..)?;
//!
//! // Read the paths from the config file and allow it to be overriden by
//! // the environment variable. We split everything at `:` to allow passing
//! // multiple paths using an environment variable. When read from the config
//! // file, multiple values can be set without using the `:` delimiter.
//! // Passing 1.. to values() makes sure at least one search path is set.
//! let search_paths: Vec<String> = conf.get(conf.root().push("searchPath")).explode(':').values(1..)?;
//!
//! # // Hide the error thrown by ?
//! # Ok::<(), justconfig::error::ConfigError>(())
//! ```
//!
//! ## Supplying defaults
//!
//! Often you want to supply default values for configuration items. This can be
//! done in two ways:
//!
//! * Use [`try_value()`](item::ValueExtractor::try_value) and
//!   supply the default by using `or`.
//! * Add a [`Defaults`](sources::defaults) source as the
//!   last configuration source to supply a default value.
//!
//! The second option shortens the pipeline length and allows the defaults to be
//! set at one central location.
//!
//! ```no_run
//! use justconfig::Config;
//! use justconfig::ConfPath;
//! use justconfig::sources::text::ConfigText;
//! use justconfig::sources::defaults::Defaults;
//! use justconfig::item::ValueExtractor;
//! use std::fs::File;
//!
//! let mut conf = Config::default();
//!
//! let config_file = File::open("myconfig.conf").expect("Could not open config file.");
//! conf.add_source(ConfigText::new(config_file, "myconfig.conf").expect("Loading configuration file failed."));
//!
//! // Add defaults for `key1` and `key2` as a fallback if they are not set via
//! // the configuration file.
//! let mut defaults = Defaults::default();
//! defaults.set(conf.root().push_all(&["key1"]), "default value 1", "default");
//! defaults.set(conf.root().push_all(&["key2"]), "default value 2", "default");
//! conf.add_source(defaults);
//! ```
//!
//!
//! ## Enumerating keys
//!
//! Every `ConfPath` keeps track of all items that where created using it. That
//! way configuration sources can create a list of configuration items that can
//! be enumerated. The [`ConfigText` source](sources::text) offers the
//! [`with_path`](sources::text::ConfigText) method
//! that allows you to pass a `ConfPath` into the parser. This `ConfPath` is
//! used to create the `ConfPath` instances for every configuration node. This
//! way the configuration can be enumerated using the passed instance.
//!
//! ```no_run
//! use justconfig::Config;
//! use justconfig::ConfPath;
//! use justconfig::sources::text::ConfigText;
//! use justconfig::sources::defaults::Defaults;
//! use justconfig::item::ValueExtractor;
//! use std::fs::File;
//!
//! let mut conf = Config::default();
//!
//! let config_file = File::open("myconfig.conf").expect("Could not open config file.");
//! let config_file_path = ConfPath::default();
//! conf.add_source(ConfigText::with_path(config_file, "myconfig.conf", &config_file_path).expect("Loading configuration file failed."));
//!
//! for config_node in config_file_path.children() {
//!     print!("{}", config_node.tail_component_name().unwrap())
//! }
//! ```

use std::default::Default;

pub mod item;
use item::StringItem;

pub mod error;
use error::ConfigError;

pub mod source;
use source::Source;

mod confpath;
pub use confpath::ConfPath;

pub mod sources;

pub mod validators;
pub mod processors;

/// Main struct representing a loaded configuration.
pub struct Config {
	sources: Vec<Box<dyn Source>>,
	path_root: ConfPath
}

impl Default for Config {
	/// Create a new configuration store.
	fn default() -> Self {
		Self {
			sources: Vec::default(),
			path_root: ConfPath::default()
		}
	}
}

impl Config {
	/// Add a configuration source to the configuration system.
	///
	/// Each configuration source must implement the [`Source`] trait.
	/// Multiple configuration sources can be added and are queried from first to last.
	/// The first configuration source that returns values for a configuration item will be used.
	/// All following configuration sources will be ignored for this configuration item.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// #
	/// let mut conf = Config::default();
	///
	/// let mut source_1 = Defaults::default();
	/// source_1.set(conf.root().push_all(&["myitem_A"]), "source_1", "source 1");
	/// conf.add_source(source_1);
	///
	/// let mut source_2 = Defaults::default();
	/// source_2.set(conf.root().push_all(&["myitem_A"]), "source_2", "source 2");
	/// source_2.set(conf.root().push_all(&["myitem_B"]), "source_2", "source 2");
	/// conf.add_source(source_2);
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem_A"])).value().unwrap();
	/// assert_eq!(value, "source_1");
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem_B"])).value().unwrap();
	/// assert_eq!(value, "source_2");
	/// ```
	pub fn add_source(&mut self, source: Box<dyn Source>) {
		self.sources.push(source);
	}

	/// Convenience method to get a ConfPath instance.
	///
	/// Can be used to get a [`ConfPath`] instance to
	/// build configuration paths. If this `ConfPath` instance is used for all
	/// calls to the configuration library all configuration values can be
	/// enumerated. For details see [`ConfPath::children()`].
	pub fn root(&self) -> ConfPath {
		self.path_root.clone()
	}

	/// Get the configuration value identified by the passed `ConfPath`.
	///
	/// This method is the root of every configuration pipeline. For usage examples
	/// see the [crates documentation](crate).
	pub fn get(&self, key: ConfPath) -> Result<StringItem, ConfigError> {
		self.sources.iter().find_map(|source| source.get(key.clone())).ok_or(ConfigError::ValueNotFound(key))
	}
}
