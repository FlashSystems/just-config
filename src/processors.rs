//! Processors trim, split, unescape or otherwise process the configuration items
//! before they get parsed into typed values.
//!
//! Processors are used to modify configuration items. For an introduction to
//! configuration items see the documentation for the [`item`](crate::item)
//! module.
//!
//! Processors are implemented as the combination of a trait and an implementation.
//! The trait defines the methods that the processor provides and is always
//! implemented for `Result<StringItem, ConfigError>`. Each processor method takes
//! an owned `self` and returns `Result<StringItem, ConfigError>`. That way
//! processors can be easily chained.
//!
//! To use a processor just put it after the [`get`](crate::Config::get)
//! method of the [`Config`](crate::Config) struct.
//!
//! ```rust
//! # use justconfig::Config;
//! # use justconfig::ConfPath;
//! # use justconfig::item::ValueExtractor;
//! # use justconfig::sources::defaults::Defaults;
//! # use justconfig::processors::Trim;
//! # let mut conf = Config::default();
//! # let mut defaults = Defaults::default();
//! defaults.set(conf.root().push_all(&["myvalue"]), "abc", "source info");
//! conf.add_source(defaults);
//!
//! let trimed_value: String = conf.get(ConfPath::from(&["myvalue"])).trim().value().unwrap();
//! ```
//!
//! ## Implementing a processor
//!
//! To implement a new processor first have a look at the [source](crate::processors)
//! of the existing processors.
//!
//! For processors there is a helper method within the
//! [`Item`](crate::item::StringItem) struct. This method is called
//! [`map`](crate::item::StringItem#map).
//!
//! The processor first checks, if there is an error value within the `Result`.
//! If there is one, the error is returned without further processing.
//! Then `map` is called and the result of the mapping operation is returned to
//! the next step of the pipeline. A basic processor looks like this:
//!
//! ```rust
//! use justconfig::error::ConfigError;
//! use justconfig::item::{StringItem, MapAction};
//!
//! pub trait Frobnify where Self: Sized {
//!   fn frobnify(self, frob_count: u8) -> Result<StringItem, ConfigError>;
//! }
//!
//! impl Frobnify for Result<StringItem, ConfigError> {
//!   fn frobnify(self, frob_count: u8) -> Result<StringItem, ConfigError> {
//!     self?.map(|v| {
//!       // Your code goes here.
//! #     MapAction::Keep
//!     })
//!   }
//! }
//! ```
use std::fmt;
use std::env;
use std::error::Error;
use crate::error::ConfigError;
use crate::item::{StringItem, MapAction};
use std::iter::FromIterator;

#[derive(Debug)]
pub enum ProcessingError {
	MissingQuotes
}

impl fmt::Display for ProcessingError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::MissingQuotes => write!(f, "value must be quoted.")
		}
	}
}

impl Error for ProcessingError {
}

/// Splits a character delimited config value into multiple configuration values.
pub trait Explode where Self: Sized {
	//TODO: Make char a pattern as soon as this is stable
	fn explode(self, delimiter: char) -> Result<StringItem, ConfigError>;
}

impl Explode for Result<StringItem, ConfigError> {
	/// Call this method on the configuration pipeline to split a config value into multiple values.
	///
	/// The passed delimiter is used as a separator for the configuration values.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Explode;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["splitme"]), "1,2,3", "source info");
	/// conf.add_source(defaults);
	///
	/// let values: Vec<u32> = conf.get(ConfPath::from(&["splitme"])).explode(',').values(..).unwrap();
	///
	/// assert_eq!(values.len(), 3);
	/// assert_eq!(values[0], 1);
	/// assert_eq!(values[1], 2);
	/// assert_eq!(values[2], 3);
	/// ```
	fn explode(self, delimiter: char) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			MapAction::Replace(Vec::from_iter(v.split(delimiter).map(|v| {
				String::from(v)
			})))
		})
	}
}

/// Trims leading, trailing or leading and trailing whitespaces from all config values.
pub trait Trim where Self: Sized {
	fn trim(self) -> Result<StringItem, ConfigError>;
	fn trim_start(self) -> Result<StringItem, ConfigError>;
	fn trim_end(self) -> Result<StringItem, ConfigError>;
}

impl Trim for Result<StringItem, ConfigError> {
	/// Trims leading and trailing whitespaces from all config values.
	///
	/// This methods calls `String::trim()` for all config values of the current
	/// configuration item.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Trim;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "   abc\t", "source info");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem"])).trim().value().unwrap();
	///
	/// assert_eq!(value, "abc");
	/// ```
	fn trim(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			if v.starts_with(char::is_whitespace) || v.ends_with(char::is_whitespace){
				MapAction::Replace(vec!(String::from(v.trim())))
			} else {
				MapAction::Keep
			}
		})
	}

	/// Trims leading whitespaces from all configuration value.
	///
	/// This methods calls `String::trim_start()` for all config values of the current
	/// configuration item.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Trim;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "   abc   ", "source info");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem"])).trim_start().value().unwrap();
	///
	/// // Note that the trailing whitespaces where kept.
	/// assert_eq!(value, "abc   ");
	/// ```
	fn trim_start(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			if v.starts_with(char::is_whitespace) {
				MapAction::Replace(vec!(String::from(v.trim_start())))
			} else {
				MapAction::Keep
			}
		})
	}

	/// Trims trailing whitespaces from all configuration value.
	///
	/// This methods calls `String::trim_end()` for all config values of the current
	/// configuration item.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Trim;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "   abc   ", "source info");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem"])).trim_end().value().unwrap();
	///
	/// // Note that the leading whitespaces where kept.
	/// assert_eq!(value, "   abc");
	/// ```
	fn trim_end(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			if v.ends_with(char::is_whitespace) {
				MapAction::Replace(vec!(String::from(v.trim_end())))
			} else {
				MapAction::Keep
			}
		})
	}
}

/// Convert escape sequences to special characters.
pub trait Unescape where Self: Sized {
	fn unescape(self) -> Result<StringItem, ConfigError>;
}

impl Unescape for Result<StringItem, ConfigError> {
	/// Call this method to convert escaped control characters to real control characters.
	///
	/// The following control characters can be used:
	///
	/// * `\n`
	/// * `\r`
	/// * `\t`
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Unescape;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), r#"\r\n"#, "source info");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["myitem"])).unescape().value().unwrap();
	///
	/// assert_eq!(value, "\r\n");
	/// ```
	fn unescape(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			let mut output = String::with_capacity(v.len() + 10);	// We assume that there are not more than 10 Escaped characters per line.

			let mut chars = v.chars();
			while let Some(c) = chars.next() {
				output.push(match c {
					'\\' => match chars.next() {
						Some('n') => '\n',
						Some('r') => '\r',
						Some('t') => '\t',
						Some(x) => x,
						None => '\\'
					}
					x => x
				});
			}

			MapAction::Replace(vec!(output))
		})
	}
}

/// Removes empty config values.
pub trait NotEmpty where Self: Sized {
	fn not_empty(self) -> Result<StringItem, ConfigError>;
}

impl NotEmpty for Result<StringItem, ConfigError> {
	/// Call this method to remove all empty configuration values from a configuration item.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::NotEmpty;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "abc", "source info");
	/// defaults.put(conf.root().push_all(&["myitem"]), "", "source info");
	/// defaults.put(conf.root().push_all(&["myitem"]), "def", "source info");
	/// conf.add_source(defaults);
	///
	/// let values: Vec<String> = conf.get(ConfPath::from(&["myitem"])).not_empty().values(..).unwrap();
	///
	/// assert_eq!(values.len(), 2);
	/// assert_eq!(values[0], "abc");
	/// assert_eq!(values[1], "def");
	/// ```
	fn not_empty(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			if v.trim().is_empty() {
				MapAction::Drop
			} else {
				MapAction::Keep
			}
		})
	}
}

/// Remove quotes from configuration strings.
pub trait Unquote where Self: Sized {
	fn unquote(self) -> Result<StringItem, ConfigError>;
}

impl Unquote for Result<StringItem, ConfigError> {
	/// Call this method to remove quotes around all configuration values.
	///
	/// All configuration values will automatically be trimmed and checked for a
	/// loading and trailing quote (`"`). If the quote is there, it will be
	/// removed. If it's missing a `ProcessingError::MissingQuotes` error will be
	/// generated.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Unquote;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["quoted"]), "\"abc\"", "source info");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["quoted"])).unquote().value().unwrap();
	///
	/// assert_eq!(value, "abc");
	/// ```
	fn unquote(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			let v = v.trim();

			if v.starts_with('"') && v.ends_with('"') {
				MapAction::Replace(vec!(v[1..v.len()-1].to_owned()))
			} else {
				MapAction::Fail(Box::new(ProcessingError::MissingQuotes))
			}
		})
	}
}

/// Type definition of a resolver function used by processors.
type Resolver<'f> = &'f dyn Fn(&str) -> Result<String, Box<dyn Error>>;

/// Expands an input string by calling a resolver function for each placeholder.
///
/// The `enabler` character starts the placeholder. The next character must be
/// the `start` character. The `end` character will terminate the placeholder.
///
/// Repeating the `enabler` character two times serves as an escape sequence to
/// allow the combination `enabler` + `start` as normal text. The duplicate
/// `enabler` character will be removed. The duplicate `enabler`character will
/// only be removed if it is followed by the start character.
///
/// The key between the `start` and the `end` character will be passed to the
/// resolver function. The result of the resolver function will be used to
/// replace the placeholder. If the resolver function returns an error, processing
/// will stop and the error will be returned.
///
/// This function will never generate an error by itself. If the `resolver`
/// function does not return an error, no error will ever be returned.
///
/// ## Example
/// If `enabler` is `$` and `start` is `{` the sequence `$${` will output `${`.
/// The sequence `$$a` will output `$$a`.
fn expand(input: &str, enabler: char, start: char, end: char, resolver: Resolver) -> Result<String, Box<dyn Error>> {
	enum EnvState { Text, ProtoPlaceholder((usize, usize)), InPlaceholder((usize, usize)), Escaped }

	let mut result = String::with_capacity(input.len());

	let mut state = EnvState::Text;

	for (pos, c) in input.char_indices() {
		if let Some(next_state) = match &state
		{
			// If we detect an enabler char in normal text we enter the
			// ProtoPlaceholder state.
			EnvState::Text if c == enabler => {
				let len_to_start = result.len();

				result.push(c);

				Some(EnvState::ProtoPlaceholder((pos, len_to_start)))
			},
			// If a second $ character appears while in ProtoPlaceholder
			// state we've detected the $$ escape and swallow the second $.
			// We advance the state to Escaped now it depends on the
			// next character what happens.
			EnvState::ProtoPlaceholder(_) if c == enabler=> {
				Some(EnvState::Escaped)
			},
			// Two $ character where detected before this { character.
			// The first $ was already put into the output stream. The
			// second one will be swallowed as an escape character.
			EnvState::Escaped if c == start => {
				result.push(c);

				Some(EnvState::Text)
			},
			// Two $ character where detected but the next character was
			// not {. We reinsert the $ sign because it was not am
			// escape character.
			EnvState::Escaped => {
				result.push(enabler);
				result.push(c);

				Some(EnvState::Text)
			},
			// If we're in proto placeholder state and get a { we are inside
			// a placeholder.
			EnvState::ProtoPlaceholder(start_pos) if c == start => {
				result.push(c);

				Some(EnvState::InPlaceholder(*start_pos))
			},
			// If we're inside a placeholder and receive a } we have reached
			// the end of the placeholder an can process it.
			EnvState::InPlaceholder(start_pos) if c == end => {
				if start_pos.0 + 2 < pos {
					result.truncate(start_pos.1);
					let value = resolver(&input[(start_pos.0 + 2)..pos])?;

					// Extend the string by the length of the value.
					// This calculation is a little strange because we have to
					// pass the number of bytes that we want to reserve in
					// addition to the current length.
					result.reserve(input.len() + value.len() - result.len());

					result.push_str(&value);
				} else {
					result.push(c);
				}

				Some(EnvState::Text)
			},
			// If we're in proto placeholder state and receive any unknown
			// character we return to text state.
			EnvState::ProtoPlaceholder(_) => {
				result.push(c);

				Some(EnvState::Text)
			},
			// Any other character is simply copied.
			_ => {
				result.push(c);
				None
			}
		} {
			// If a new state was returned, update the state variable
			state = next_state;
		}
	}

	Ok(result)
}

/// Substitute placeholders within config values with values (for example
/// environment variables).
pub trait Subst where Self: Sized {
	fn env(self) -> Result<StringItem, ConfigError>;
	fn expand(self, start: char, end: char, resolver: Resolver) -> Result<StringItem, ConfigError>;
}

impl Subst for Result<StringItem, ConfigError> {
	/// Call this method to substitute placeholders with environment variables.
	///
	/// An environment variable can be referenced by `${name}`. Every occurrence of
	/// of this placeholder is expanded by replacing it with the named environment
	/// variable. If the environment variable is not set or can not be converted into
	/// a UTF-8 string an empty string is substituted.
	///
	/// To escape the start sequence `${` a second `$` character must be used.
	/// For example `$${LITERAL}` will be replaced by `${LITERAL}` without expanding
	/// the environment variable `LITERAL`. Any `$` character not followed by `{`
	/// must not be escaped. The string `cash: $$$` will be returned as `cash: $$$`.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Subst;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["env"]), "${PATH}", "substitute PATH");
	/// conf.add_source(defaults);
	///
	/// let value: String = conf.get(ConfPath::from(&["env"])).env().value().unwrap();
	///
	/// assert_eq!(value, std::env::var("PATH").unwrap_or_default());
	/// ```
	fn env(self) -> Result<StringItem, ConfigError> {
		self?.map(|v| {
			// Unwrap can be called here because we always return ok from the resolver closure.
			// ToDo: Use into_ok() as soon as it's stable.
			let result = expand(v, '$', '{', '}', &|key| { Ok(env::var(key).unwrap_or_default()) } ).unwrap();

			MapAction::Replace(vec!(result))
		})
	}

	/// Call this method to substitute placeholders with an application defined
	/// value.
	///
	/// For a general description see the [`env`](crate::processors::Subst::env) method. This
	/// method is more general than `env` as it allows the characters enclosing
	/// the variable to be set and uses a callback to supply the value that
	/// should be substituted.
	///
	/// Because this function allows the enclosing characters to be set
	/// different substitutions can be used for different sources of the
	/// substituted value. For example `${}` can be sued for environment
	/// variable substitution and `$()` could be used for substitution for a
	/// secondary configuration file.
	/// 
	/// The `$` character as the start marker can not be changed.
	///
	/// ## Example
	///
	/// This example emulates the [`env`](crate::processors::Subst::env) method but returns an
	/// error if the environment variable is not found. In addition it replaces
	/// the curly brackets used by the `env` method with round ones.
	///
	/// ```rust
	/// # use std::error::Error;
	/// # use std::env;
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::error::ConfigError;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::processors::Subst;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["env"]), "$(I_DONT_KONW)", "substitute PATH");
	/// conf.add_source(defaults);
	///
	/// let result: Result<String, ConfigError> = conf.get(ConfPath::from(&["env"])).expand('(', ')', &|key| { env::var(key).map_err(Box::from) } ).value();
	///
	/// assert!(result.is_err());
	/// ```
	fn expand(self, start: char, end: char, resolver: &dyn Fn(&str) -> Result<String, Box<dyn Error>>) -> Result<StringItem, ConfigError> {
		assert_ne!(start, '$');
		assert_ne!(end, '$');

		self?.map(|v| {
			// Unwrap can be called here because we always return ok from the resolver closure.
			match expand(v, '$', start, end, resolver) {
				Ok(result) => MapAction::Replace(vec!(result)),
				Err(error) => MapAction::Fail(error)
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Config;
	use crate::confpath::ConfPath;
	use crate::item::ValueExtractor;
	use crate::sources::defaults::Defaults;
	use crate::error::ConfigError;

	#[test]
	fn explode() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["empty"]), "", "empty_test");
		d.set(c.root().push_all(["ten"]), "10", "10");
		d.set(c.root().push_all(["splitme"]), "1,2,3", "splitme");
		d.set(c.root().push_all(["multisplit"]), "1:2", "multisplit.1");
		d.put(c.root().push_all(["multisplit"]), "3:4:5", "multisplit.2");
		c.add_source(d);

		let values: Vec<u32> = c.get(ConfPath::from(&["splitme"])).explode(',').values(..).unwrap();

		assert_eq!(values.len(), 3);
		assert_eq!(values[0], 1);
		assert_eq!(values[1], 2);
		assert_eq!(values[2], 3);

		let values: Vec<u32> = c.get(ConfPath::from(&["ten"])).explode(',').values(..).unwrap();

		assert_eq!(values.len(), 1);
		assert_eq!(values[0], 10);

		let values: Vec<String> = c.get(ConfPath::from(&["empty"])).explode(',').values(..).unwrap();

		assert_eq!(values.len(), 1);
		assert!(values[0].is_empty());

		let values: Vec<u32> = c.get(ConfPath::from(&["multisplit"])).explode(':').values(..).unwrap();

		assert_eq!(values.len(), 5);
		assert_eq!(values[0], 1);
		assert_eq!(values[1], 2);
		assert_eq!(values[2], 3);
		assert_eq!(values[3], 4);
		assert_eq!(values[4], 5);
	}

	#[test]
	fn trim() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["trim"]), "  text  ", "splitme");
		d.set(c.root().push_all(["trim_mixed"]), "\ttext  ", "splitme");
		c.add_source(d);

		let value: String = c.get(ConfPath::from(&["trim"])).trim().value().unwrap();
		assert_eq!(value, "text");

		let value: String = c.get(ConfPath::from(&["trim"])).trim_start().value().unwrap();
		assert_eq!(value, "text  ");

		let value: String = c.get(ConfPath::from(&["trim"])).trim_end().value().unwrap();
		assert_eq!(value, "  text");

		let value: String = c.get(ConfPath::from(&["trim_mixed"])).trim().value().unwrap();
		assert_eq!(value, "text");
	}

	#[test]
	fn unescape() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["standard"]), "\\r\\n\\t", "standard");
		d.set(c.root().push_all(["with_text"]), "rrr\\rnnn\\nttt\\t", "standard");
		d.set(c.root().push_all(["unknown"]), "\\x\\y\\z", "unknown");
		d.set(c.root().push_all(["at_end"]), "Text\\", "at_end");
		c.add_source(d);

		let value: String = c.get(ConfPath::from(&["standard"])).unescape().value().unwrap();
		assert_eq!(value, "\r\n\t");

		let value: String = c.get(ConfPath::from(&["with_text"])).unescape().value().unwrap();
		assert_eq!(value, "rrr\rnnn\nttt\t");

		let value: String = c.get(ConfPath::from(&["unknown"])).unescape().value().unwrap();
		assert_eq!(value, "xyz");

		let value: String = c.get(ConfPath::from(&["at_end"])).unescape().value().unwrap();
		assert_eq!(value, "Text\\");
	}

	#[test]
	fn not_empty() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["some_empty"]), "some_empty", "some_empty1");
		d.put(c.root().push_all(["some_empty"]), "", "some_empty2");
		d.put(c.root().push_all(["some_empty"]), " ", "some_empty3");
		d.put(c.root().push_all(["some_empty"]), "not_empty", "some_empty4");
		d.set(c.root().push_all(["all_empty"]), "", "all_empty1");
		d.put(c.root().push_all(["all_empty"]), "", "all_empty2");
		c.add_source(d);

		let mut values: Vec<String> = c.get(ConfPath::from(&["some_empty"])).not_empty().values(..).unwrap();
		assert_eq!(values.len(), 2);
		assert_eq!(values.pop().unwrap(), "not_empty");
		assert_eq!(values.pop().unwrap(), "some_empty");


		let values: Vec<String> = c.get(ConfPath::from(&["all_empty"])).not_empty().values(..).unwrap();
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn unquote() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["quote"]), "\"test\"", "quote");
		d.set(c.root().push_all(["no_quote"]), "test", "no_quote");
		d.set(c.root().push_all(["start_quote"]), "\"test", "start_quote");
		d.set(c.root().push_all(["end_quote"]), "test\"", "end_quote");
		c.add_source(d);

		let value: String = c.get(ConfPath::from(&["quote"])).unquote().value().unwrap();
		assert_eq!(value, "test");

		assert!((c.get(ConfPath::from(&["no_quote"])).unquote().value() as Result<String, ConfigError>).is_err());
		assert!((c.get(ConfPath::from(&["start_quote"])).unquote().value() as Result<String, ConfigError>).is_err());
		assert!((c.get(ConfPath::from(&["end_quote"])).unquote().value() as Result<String, ConfigError>).is_err());
	}

	#[test]
	#[should_panic(expected = "MissingQuotes")]
	fn unquote_error() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["missing_end_quote"]), "\"test", "start_quote");
		c.add_source(d);

		let _: String = c.get(ConfPath::from(&["missing_end_quote"])).unquote().value().unwrap();
	}

	#[test]
	fn env() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["env"]), "env=${TEST_ENV}", "env");
		d.set(c.root().push_all(["env_multiple"]), "a_first=${TEST_ENV} b_second=${TEST_ENV_SECOND} c_missing=${MISSING_ENV}", "env_multiple");
		d.set(c.root().push_all(["env_missing"]), "env=${MISSING_ENV}", "missing_env");
		d.set(c.root().push_all(["env_empty"]), "env=${}", "env_test");
		d.set(c.root().push_all(["env_escape"]), "env=$${NO_REPLACE}", "env_escape");
		d.set(c.root().push_all(["env_fake_escape"]), "cash=20$$$", "env_fake_escape");
		d.set(c.root().push_all(["env_unclosed"]), "env=${UNCLOSED", "env_unclosed");
		d.set(c.root().push_all(["env_special"]), "env=${${ENV}}", "env_special");
		c.add_source(d);

		env::set_var("TEST_ENV", "asdf");
		env::set_var("TEST_ENV_SECOND", "xyz");

		let value: String = c.get(ConfPath::from(&["env"])).env().value().unwrap();
		assert_eq!(value, "env=asdf");
		let value: String = c.get(ConfPath::from(&["env_multiple"])).env().value().unwrap();
		assert_eq!(value, "a_first=asdf b_second=xyz c_missing=");
		let value: String = c.get(ConfPath::from(&["env_missing"])).env().value().unwrap();
		assert_eq!(value, "env=");
		let value: String = c.get(ConfPath::from(&["env_empty"])).env().value().unwrap();
		assert_eq!(value, "env=${}");

		let value: String = c.get(ConfPath::from(&["env_escape"])).env().value().unwrap();
		assert_eq!(value, "env=${NO_REPLACE}");
		let value: String = c.get(ConfPath::from(&["env_fake_escape"])).env().value().unwrap();
		assert_eq!(value, "cash=20$$$");

		let value: String = c.get(ConfPath::from(&["env_unclosed"])).env().value().unwrap();
		assert_eq!(value, "env=${UNCLOSED");
		let value: String = c.get(ConfPath::from(&["env_special"])).env().value().unwrap();
		assert_eq!(value, "env=}");
	}

	#[test]
	fn expand() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["round_br"]), "env=$(TEST)", "round_br");
		d.set(c.root().push_all(["square_br"]), "env=$[TEST]", "square_br");
		d.set(c.root().push_all(["same_start_end"]), "env=$|TEST|", "same_start_end");
		c.add_source(d);

		// This resolver checks if the passed key is "TEST". All tests use this key.
		let resolver_ok = |key: &str| { assert_eq!(key, "TEST"); Ok(String::from("asdf")) };
		let resolver_err: Resolver = &|_: &str| { Err(Box::new(std::env::VarError::NotPresent)) };

		let value: String = c.get(ConfPath::from(&["round_br"])).expand('(', ')', &resolver_ok).value().unwrap();
		assert_eq!(value, "env=asdf");
		let value: String = c.get(ConfPath::from(&["square_br"])).expand('[', ']', &resolver_ok).value().unwrap();
		assert_eq!(value, "env=asdf");
		let value: String = c.get(ConfPath::from(&["same_start_end"])).expand('|', '|', &resolver_ok).value().unwrap();
		assert_eq!(value, "env=asdf");

		assert!((c.get(ConfPath::from(&["round_br"])).expand('(', ')', resolver_err).value() as Result<String, ConfigError>).is_err());
	}

	#[test]
	fn self_resolve() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["expand_me"]), "env=${test}", "round_br");
		d.set(c.root().push_all(["test"]), "asdf", "square_br");
		c.add_source(d);

		// This resolver uses the config tree to resolve the passed key. That way the config system can refer to itself
		let resolver = |key: &str| { (c.get(c.root().push_all(key.split('.'))).value() as Result<String, ConfigError>).map_err(Box::from) };

		let value: String = c.get(ConfPath::from(&["expand_me"])).expand('{', '}', &resolver).value().unwrap();
		assert_eq!(value, "env=asdf");
	}
}
