//! Text source.
//! 
//! This sources parses a text representation of the configuration.
//! The configuration format is as simple as possible to make it as easy to
//! write and parse as possible.
//! 
//! Any struct that implements `Read` can be used as a source for configuration
//! values. The text must be UTF-8 encoded.
//! 
//! ```no_run
//! use justconfig::Config;
//! use justconfig::sources::text::ConfigText;
//! use std::fs::File;
//! 
//! let mut conf = Config::default();
//! 
//! let file = File::open("myconfig.conf").expect("Could not open config file.");
//! let conf_file = ConfigText::new(file, "myconfig.conf").unwrap();
//! conf.add_source(conf_file);
//! ```
//! 
//! The name of the configuration source must be passed as a second parameter. It
//! is used to create nice error messages to show to the user.
//! 
//! ## Stacking configuration files
//! 
//! To search for configuration information in multiple directories the convenience
//! function [`stack_config`](fn.stack_config.html) is provided. It
//! merges all found configuration files allowing parts of a default configuration
//! file to be overwritten by more specific configuration files in other
//! directories.
//! 
//! ## Configuration Format
//! 
//! The configuration consists primarily of key-value-pairs. On a normal configuration
//! line everything before the first equals sign (`=`) is considered a key. Everything
//! after the equals sign is the value.
//! 
//! The configuration file format does not make any assumptions about the format of the
//! value. The usage of quotes or validation can be declared by using
//! [processors](FIXME) or [validators](FIXME).
//! 
//! Just-Config allows configuration keys to be organized in a hierarchical manner.
//! This is represented by a [ConfPath](../../struct.ConfPath.html) structure. To represent hierarchical
//! configuration values within the key value of the text configuration dot (`.`) is
//! used as a delimiter.
//! 
//! ```conf
//! key=value
//! section.subsection.key=value
//! ```
//! 
//! This configuration file defines two values with the following ConfPath:
//! 
//! ```rust
//! # use justconfig::ConfPath;
//! ConfPath::from(&["key"]);
//! ConfPath::from(&["section", "subsection", "key"]);
//! ```
//!  
//! Leading white-spaces before the key and any white-space between the key and the
//! equals sign are ignored. White spaces *after* the equals sign *are* significant.
//! 
//! ## Comments
//! 
//! Everything on a line after the first hash character (`#`) is ignored. Comments
//! can be put in front of the line, making the whole line a comment or after non
//! comment text:
//! 
//! ```conf
//! ## Complete comment
//! key=value # Comment
//! [Section] # Comment
//! ```
//! 
//! To include a literal hash character into a value or key it has to be escaped
//! by prepending it with a backslash (`\`):
//! 
//! ```conf
//! key=value containing \#hash
//! ```
//! 
//! ## Sections
//! 
//! Sections can be used to prevent typing the same prefixes for keys over and
//! over. Every line where the first, non white-space character is a square bracket
//! (`[`) is considered a section header. The section header can contain one
//! or more prefixes to put in front of every key that follows the section header:
//! 
//! ```conf
//! [section]
//! key=value
//! # Key is section.key=value
//! 
//! [section.subsection]
//! key=value
//! # Key is section.subsection.key=value
//! 
//! [section]
//! subsection.key=value
//! # Key is section.subsection.key=value
//! ```
//! 
//! ## Multiple values per key
//! 
//! A key can have multiple values. Just assign multiple values to the same to use
//! this feature. To make typing a little less tedious, the key can be omitted if
//! subsequent values are assigned to the same key:
//! 
//! ```conf
//! key=value1
//! key=value2
//! 
//! # is the same as
//! 
//! key=value1
//!    =value2
//! ```
//! 
//! White-space characters before the equals sign are ignored. That way indentation
//! is possible.
//! 
//! *Beware*: An empty line (or a line only containing a comment) resets the current
//! key. The following configuration will return a `NoPreviousKey` error:
//! 
//! ```conf
//! key=value1
//! # I'm a comment but an empty line will do, too.
//! =value2
//! ```
//! 
//! ## Multi line values
//! 
//! Sometimes a value should span multiple lines. There are two possible methods to
//! achieve this:
//! 
//! * Using the [`unescape`](FIXME) Processor.
//! * Using line continuation.
//! 
//! The line continuation feature of the text parser allows lines to be continued.
//! In contrast to other configuration file formats the line continuation has to be
//! introduced on the continuing line, *not* on the line that is continued. To
//! continue a line the first non white-space character must be the pipe character
//! (`|`).
//! 
//! ```conf
//! key=line1
//!    |line2
//! ```
//! 
//! The continuation character can even be indented to make the multi-line configuration
//! entry easier to read.
//! 
//! The second line is appended to the first line after a newline character (`\n`).
//! 
use crate::source::Source;
use crate::item::{SourceLocation, StringItem, Value};
use crate::confpath::ConfPath;
use crate::Config;

use std::io::{Read, BufRead, BufReader};
use std::path::Path;
use std::fs::File;
use std::ffi::OsString;
use std::collections::HashMap;
use std::rc::Rc;
use std::fmt;

/// Enumeration containing parse errors.
#[derive(Debug)]
pub enum Error {
	/// The first none white-space character on the line was an equals sign (`=`)
	/// but there was no previous line that set the key.
	NoPreviousKey(Rc<TextSourceLocation>),
	/// A line was found that is not a section header and not a continuation of the
	/// previous line but misses the key-value-delimiter (`=`).
	MissingKeyValueDelimiter(Rc<TextSourceLocation>),
	/// An I/O error occurred while reading.
	IoError(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Error::NoPreviousKey(location) => write!(f, "No previous key in {}", location),
			Error::MissingKeyValueDelimiter(location) => write!(f, "Missing value for key in {}", location),
			Error::IoError(error) => write!(f, "I/O error: {}", error),
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::IoError(source) => Some(source),
			_ => None
		}
	}
}

impl From<std::io::Error> for Error {
	fn from(io_error: std::io::Error) -> Self {
		Error::IoError(io_error)
	}
}

/// Source location for the ConfigText configuration source.
/// This value is used to store the source of every configuration value for
/// use in error messages.
#[derive(Debug)]
pub struct TextSourceLocation {
	source_name: String,
	line_start: usize,
	line_end: usize
}

impl TextSourceLocation {
	fn new(source_name: &str, line_start: usize, line_end: usize) -> Rc<Self> {
		Rc::new(Self {
			source_name: source_name.to_owned(),
			line_start,
			line_end
		})
	}
}

impl fmt::Display for TextSourceLocation {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.line_start == self.line_end {
			write!(f, "conf:{}:{}", self.source_name, self.line_start)
		} else {
			write!(f, "conf:{}:{}-{}", self.source_name, self.line_start, self.line_end)
		}
	}
}

impl SourceLocation for TextSourceLocation {}

struct CurrentValue<'a> {
	value: String,
	source_name: &'a str,
	line_start: usize,
	line_end: usize
}

/// Implements the text configuration parser.
pub struct ConfigText {
	items: HashMap<ConfPath, StringItem>
}

impl ConfigText {
	fn put_value(&mut self, key: &Option<ConfPath>, value: &mut Option<CurrentValue>) {
		if let Some(key) = key {
			if let Some(value) = value.take() {
				self.items.entry(key.clone()).or_insert_with(|| StringItem::new(key.clone())).push(Value::new(value.value, TextSourceLocation::new(value.source_name, value.line_start, value.line_end)));
			} else {
				unreachable!("Logic error: put_value must not be called without a current value.");
			}
		}
	}

	fn find_start_of_comment(s: &str) -> Option<usize> {
		let mut chars = s.chars();

		let mut pos = 0;
		while let Some(c) = chars.next() {
			match c {
				'#' => return Some(pos),
				'\\' => { chars.next(); pos+=1; },
				_ => ()
			}

			pos+=1;
		}

		None
	}

	/// Parses a text representation into configuration information.
	/// 
	/// Any instance of a struct implementing `Read` can be passed to the configuration
	/// parser. As the second parameter a string identifying the configuration source
	/// must be passed. This string is used to construct the error location when
	/// displaying error messages.
	/// 
	/// The method returns a new `ConfigText` instance or an error if the file could not
	/// be parsed.
	pub fn new(conf_source: impl Read, source_name: &str) -> Result<Box<Self>, Error> {
		Self::with_path(conf_source, source_name, &ConfPath::default())
	}

	/// Parse a text representation of configuration information and fill a `ConfPath`
	/// with the contained keys. 
	/// 
	/// To be able to enumerate the keys of a configuration the
	/// [`children`](../../struct.ConfPath.html#method.children) method of a
	/// [`ConfPath`](../../struct.ConfPath.html) instance must be used. This variant of the
	/// [`new`](#method.new) method allows a `ConfPath` instance to be passed. This 
	/// instance is used to construct all configuration paths while parsing the text
	/// representation. After this method returns the `ConfPath` instance can be used
	/// to explore the contents of the parsed text configuration.
	pub fn with_path(conf_source: impl Read, source_name: &str, path_root: &ConfPath) -> Result<Box<Self>, Error> {
		let mut conf = Self {
			items: HashMap::default()
		};

		let reader = BufReader::new(conf_source);

		let mut current_key: Option<ConfPath> = None;
		let mut current_value: Option<CurrentValue> = None; // We've to store the TextSourceLocation as well to update it on multi line items.
		let mut current_section = path_root.clone();

		let mut line_no: usize = 1;
		for read_line in reader.lines() {
			let mut line = read_line?;

			// Detect comments and remove them
			if let Some(pos) = Self::find_start_of_comment(&line) {
				line.truncate(pos);
			}

			let trimed = line.trim();
			if trimed.is_empty() {
				// Empty lines reset the current key. A line continuation after an empty line is impossible.
				conf.put_value(&current_key, &mut current_value);
				current_key = None;
			} else if trimed.starts_with('[') && trimed.ends_with(']') {
				conf.put_value(&current_key, &mut current_value);

				// Update the current section if a section header was found
				current_section=path_root.push_all(trimed.trim()[1..trimed.len()-1].split('.'));

				// Reset the current key, because we're within an new section
				current_key = None;
			} else if trimed.starts_with('|') {
				// If the first, non white-space character on the line is the line continuation
				// character the line is appended to the previous line after adding a newline character.
				if current_key.is_some() {
					let mut current_value_mut = current_value.take().unwrap(); // We unwrap here because current_key is_some and then current_value must be some, too.

					if !current_value_mut.value.is_empty() { current_value_mut.value.push('\n'); }
					current_value_mut.value.push_str(&line.trim_start()[1..]);

					current_value_mut.line_end = line_no;

					current_value = Some(current_value_mut);
				} else {
					return Err(Error::NoPreviousKey(TextSourceLocation::new(source_name, line_no, line_no)));
				}

			} else {
				conf.put_value(&current_key, &mut current_value);

				// The line does not start with a white-space or the first character after
				// the white-space(s) is an equals sign
				if let [key, value] = line.splitn(2, '=').collect::<Vec<&str>>()[..] {
					let key = key.trim();

					// If there is a key then we set this key as the current key
					if !key.is_empty() {
						current_key = Some(current_section.push_all(key.trim().split('.')));
					}

					// Check if there was a previous key.
					// If there wasn't a previous key the user tries to add a
					// value to a key that does not exist.
					if current_key.is_none() {
						return Err(Error::NoPreviousKey(TextSourceLocation::new(source_name, line_no, line_no)));
					}

					// Save the value for later
					current_value = Some(CurrentValue {
						value: value.to_owned(),
						source_name,
						line_start: line_no,
						line_end: line_no
					});
				} else {
					return Err(Error::MissingKeyValueDelimiter(TextSourceLocation::new(source_name, line_no, line_no)));
				}
			}

			line_no+=1;
		}

		// Final put if there is a value pending
		if current_value.is_some() {
			conf.put_value(&current_key, &mut current_value);
		}

		Ok(Box::new(conf))
	}
}

impl Source for ConfigText {
	fn get(&self, key: ConfPath) -> Option<StringItem> {
		self.items.get(&key).cloned()
	}
}

/// Helper function for config file stacking.
///
/// This function is a helper to allow searching for a configuration file in
/// multiple directories. Often a default configuration is supplied by the
/// distribution within `/usr/share/mypackage` and the administrator can
/// override some settings by supplying a configuration file in `/etc`. Maybe
/// even the user should be able to change some configuration options by having
/// a third configuration file within its home directory.
///
/// This helper function searches the given list of paths in the supplied order
/// and adds every configuration file as a configuration source. That way the
/// configuration files are merged by the rules stated in
/// [`addSource`](../../struct.Config.html#method.add_source).
///
/// The `config_path` parameter allows you to borrow a
/// [`ConfPath`](../../struct.ConfPath.html) instance to the function. This instnace
/// will be used to store all configuration values for enumeration. See
/// [Enumerating keys](../../index.html#enumerating-keys) for details.
///
/// ## Example
///
/// ```rust
/// # use std::path::Path;
/// # use std::ffi::OsString;
/// # use justconfig::Config;
/// # use justconfig::sources::text::{ConfigText, stack_config};
///
/// // Define the search path.
/// let paths: [&Path; 3] = [
///   &Path::new("/usr/share/myapp/etc"),
///   &Path::new("/etc"),
///   &Path::new(env!("HOME")).join(".config").join("myapp")
/// ];
///
/// let mut config = Config::default();
/// stack_config(&mut config, None, &OsString::from("myapp.conf"), &paths[..]).unwrap();
/// ```
pub fn stack_config(config: &mut Config, config_path: Option<&mut ConfPath>, file_name: &OsString, paths: &[&Path]) -> Result<(), Error>{
	let file_name = Path::new(file_name);

	for &path in paths {
		let this_path = path.join(file_name);
		if let Ok(config_file) = File::open(&this_path) {
			if let Some(config_path) = &config_path {
				config.add_source(ConfigText::with_path(config_file, &this_path.to_string_lossy(), *config_path)?);
			} else {
				config.add_source(ConfigText::new(config_file, &this_path.to_string_lossy())?);
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::source::Source;
	use crate::item::StringItem;
	use crate::item::ValueExtractor;
	use crate::error::ConfigError;

	fn assert_item(items: StringItem, template: &[&str]) {
		// This construction generates a Result-Value that can be used with the ValueExtractor-Trait-Impl to get the values of the StringItem.
		// It's a little bit convoluted but makes sure we use the standard tooling for the test.
		let items: Vec<String> = Ok(Result::<StringItem, ConfigError>::Ok(items).unwrap()).values(..).unwrap();
		let mut items_iter = items.iter();
		let mut tmpl_iter = template.iter();

		while let (Some(a), Some(b)) = (items_iter.next(), tmpl_iter.next()) {
			assert_eq!(a, *b);
		}
	}

	#[test]
	fn parsing() {
		let config_file = r#"
key1=value1
key2=value2.1
key2=value2.2
key3=value3.1
	|value3.2
|value3.3
key4=value4.1
	=value4.2
	key5=value5
test2.key6=value6

[test1]
key1=value1
=value2

[comments] # My comment
key1=value # comment
key2=value#comment
key3=value\#nocomment
key4=value\#nocomment # comment
"#;

		let conf = ConfigText::new(config_file.as_bytes(), "myfile").unwrap();

		assert_item(conf.get(ConfPath::from(["key1"])).unwrap(), &["value1"]);
		assert_item(conf.get(ConfPath::from(["key2"])).unwrap(), &["value2.1", "value2.2"]);
		assert_item(conf.get(ConfPath::from(["key3"])).unwrap(), &["value3.1\nvalue3.2\nvalue3.3"]);
		assert_item(conf.get(ConfPath::from(["key4"])).unwrap(), &["value4.1", "value4.2"]);
		assert_item(conf.get(ConfPath::from(["key5"])).unwrap(), &["value5"]);
		assert_item(conf.get(ConfPath::from(["test2", "key6"])).unwrap(), &["value6"]);

		assert_item(conf.get(ConfPath::from(["test1", "key1"])).unwrap(), &["value1", "value2"]);

		assert_item(conf.get(ConfPath::from(["comments", "key1"])).unwrap(), &["value "]);
		assert_item(conf.get(ConfPath::from(["comments", "key2"])).unwrap(), &["value"]);
		assert_item(conf.get(ConfPath::from(["comments", "key3"])).unwrap(), &["value\\#nocomment"]);
		assert_item(conf.get(ConfPath::from(["comments", "key4"])).unwrap(), &["value\\#nocomment "]);
	}

	#[test]
	#[should_panic(expected = "NoPreviousKey(TextSourceLocation { source_name: \"myfile\", line_start: 2, line_end: 2 })")]
	fn prase_error_dangling_cont() {
		let config_file = r#"
|Continuation without value.
"#;

		let _ = ConfigText::new(config_file.as_bytes(), "myfile").unwrap();
	}

	#[test]
	#[should_panic(expected = "MissingKeyValueDelimiter(TextSourceLocation { source_name: \"myfile\", line_start: 2, line_end: 2 })")]
	fn prase_error_dangling_key() {
		let config_file = r#"
Key without value
"#;

		let _ = ConfigText::new(config_file.as_bytes(), "myfile").unwrap();
	}

	#[test]
	#[should_panic(expected = "NoPreviousKey(TextSourceLocation { source_name: \"myfile\", line_start: 2, line_end: 2 })")]
	fn prase_error_dangling_value() {
		let config_file = r#"
=Add without key
"#;

		let _ = ConfigText::new(config_file.as_bytes(), "myfile").unwrap();
	}

	#[test]
	fn stack() {
		let paths: [&Path; 3] = [
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p1"),
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p_non"),
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p2")
		];

		let mut config = Config::default();
		stack_config(&mut config, None, &OsString::from("test.conf"), &paths[..]).unwrap();

		assert_eq!((config.get(ConfPath::from(["key_p1"])).value() as Result<String, ConfigError>).unwrap(), "p1");
		assert_eq!((config.get(ConfPath::from(["key_p2"])).value() as Result<String, ConfigError>).unwrap(), "p2");
		assert_eq!((config.get(ConfPath::from(["key_p1_p2"])).value() as Result<String, ConfigError>).unwrap(), "p1");
	}

	#[test]
	fn stack_with_path() {
		let paths: [&Path; 3] = [
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p1"),
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p_non"),
			&Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("p2")
		];

		let mut config = Config::default();
		let mut cp = ConfPath::default();
		stack_config(&mut config, Some(&mut cp), &OsString::from("test.conf"), &paths[..]).unwrap();

		// Check that all children are there.
		// The iterator is not sorted so we've to sort later.
		let mut key_names: Vec<_> = cp.children().map(|c| String::from(c.tail_component_name().unwrap())).collect();
		key_names.sort();

		assert_eq!(key_names.len(), 3);
		assert_eq!(key_names[0], "key_p1");
		assert_eq!(key_names[1], "key_p1_p2");
		assert_eq!(key_names[2], "key_p2");
	}
}
