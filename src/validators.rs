//! Validators, in contrast to [processors](crate::processors) do not
//! modify the value on any way.
//!
//! They are called after processors and operate on
//! the already typed configuration value. Therefore they can do things, that
//! processors cant. They can do validation on attributes of the typed value
//! (less than for example) that are not easily possible on strings.
//!
//! Validators are used to validate the contents of the values of a configuration
//! item. For an introduction to configuration items see the documentation for
//! the [`item`](crate::item) module.
//!
//! Validators are implemented as the combination of a trait and an implementation.
//! The trait defines the methods that the processor provides and is always
//! implemented for `Result<TypedItem<T>, ConfigError>` and
//! `Result<StringItem, ConfigError>`. Each processor method takes
//! an owned `self` and returns `Result<TypedItem<T>, ConfigError>`. That way
//! processors can be easily chained.
//!
//! The implementation for `Result<StringItem, ConfigError>` is only a wrapper
//! that does the type conversion from `String` to `T` on the first validator
//! call. All validator further down the pipeline operate directly on the converted
//! value.
//!
//! To use a validator just put directly after the
//! [`get`](crate::Config::get) method of the
//! [`Config`](crate::Config) struct or after the last processor.
//!
//! ```rust
//! # use justconfig::Config;
//! # use justconfig::ConfPath;
//! # use justconfig::error::ConfigError;
//! # use justconfig::item::ValueExtractor;
//! # use justconfig::sources::defaults::Defaults;
//! # use justconfig::validators::Range;
//! # let mut conf = Config::default();
//! # let mut defaults = Defaults::default();
//! defaults.set(conf.root().push_all(&["myvalue"]), "4", "source info");
//! conf.add_source(defaults);
//!
//! let at_most_five: i16 = conf.get(ConfPath::from(&["myvalue"])).max(5).value().unwrap();
//!
//! // This will fail because 4 is more that 3.
//! let at_most_three: Result<i16, ConfigError> = conf.get(ConfPath::from(&["myvalue"])).max(3).value();
//! assert!(at_most_three.is_err());
//! ```
//!
//! ## Implementing a validator
//!
//! To implement a new validator first have a look at the [source](crate::validators)
//! of the existing validators.
//!
//! For validators there is a helper method within the
//! [`TypedItem`](crate::item::TypedItem) struct. This method is called
//! [`filter`](crate::item::TypedItem::filter).
//!
//! The validator first checks, if there is an error value within the `Result`.
//! If there is one, the error is returned without any further validation.
//! Then `filter` is called and the result of the filtering operation is returned to
//! the next step of the pipeline. A basic validator looks like this:
//!
//! ```rust
//! use std::str::FromStr;
//! use std::convert::TryInto;
//! use std::error::Error;
//! use justconfig::error::ConfigError;
//! use justconfig::item::{StringItem, TypedItem, MapAction};
//!
//! pub trait IsFrobable<T: FromStr> {
//!   fn isFrobable(self, max_frobability: u8) -> Result<TypedItem<T>, ConfigError>;
//! }
//!
//! impl <T: FromStr> IsFrobable<T> for Result<TypedItem<T>, ConfigError> {
//!   fn isFrobable(self, max_frobability: u8) -> Result<TypedItem<T>, ConfigError> {
//!     self?.filter(|v| {
//!       // Your code goes here.
//! #     Ok(())
//!     })
//!   }
//! }
//!
//! // This is the necessary wrapper to ensure the conversion of StringItem to
//! // to TypedItem<T>.
//! impl <T: FromStr> IsFrobable<T> for Result<StringItem, ConfigError> where T::Err: Error + 'static {
//!   fn isFrobable(self, max_frobability: u8) -> Result<TypedItem<T>, ConfigError> {
//!     (self.try_into() as Result<TypedItem<T>, ConfigError>).isFrobable(max_frobability)
//!   }
//! }
//! ```
//! This example shows the necessary wrapper that allows a validator to be called
//! on the string- and typed-value.
//!
//! The type argument `T` your validator must be bound to implement the trait `FromStr`.
//! This is necessary to allow the conversion from `StringItem` to `TypedItem<T>`. If
//! your validator needs `T` to be bound to multiple traits just add them after the
//! 'FromStr' trait using the `+` syntax.
use std::fmt;
use std::error::Error;
use std::str::FromStr;
use std::convert::TryInto;
use std::ops::RangeBounds;

use crate::error::ConfigError;
use crate::item::{StringItem, TypedItem};

#[derive(Debug)]
pub enum ValidatorError {
	Empty,
	BelowMinimum(String),
	AboveMaximum(String),
	NotInRange(Option<String>, Option<String>)
}

impl fmt::Display for ValidatorError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Empty => write!(f, "must not be empty."),
			Self::BelowMinimum(min) => write!(f, "must be >= {}.", min),
			Self::AboveMaximum(max) => write!(f, "must be <= {}.", max),
			Self::NotInRange(start, end) => {
				write!(f, "must be ")?;
				if let Some(start) = start {
					write!(f, "{}.", start)?;
				}
				if start.is_some() && end.is_some() {
					write!(f, " and ")?;
				}
				if let Some(end) = end {
					write!(f, "{}.", end)?;
				}
				Ok(())
			}
		}
	}
}

impl Error for ValidatorError {
}

impl ValidatorError {
	fn from_range<T: fmt::Display, R: RangeBounds<T>>(range: &R) -> Self {
		let start = match range.start_bound() {
			std::ops::Bound::Included(v) => { Some(format!(">= {}", v)) },
			std::ops::Bound::Excluded(v) => { Some(format!("> {}", v)) },
			std::ops::Bound::Unbounded => { None}
		};

		let end = match range.end_bound() {
			std::ops::Bound::Included(v) => { Some(format!("<= {}", v)) },
			std::ops::Bound::Excluded(v) => { Some(format!("< {}", v)) },
			std::ops::Bound::Unbounded => { None }
		};

		Self::NotInRange(start, end)
	}
}

/// Validates if a configuration value is within range by using the
/// [`PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html)
/// trait of the configuration target value.
pub trait Range<T: FromStr + PartialOrd + fmt::Display> {
	fn min(self, minimum: T) -> Result<TypedItem<T>, ConfigError>;
	fn max(self, maximum: T) -> Result<TypedItem<T>, ConfigError>;
	fn in_range<R: RangeBounds<T>>(self, range: R) -> Result<TypedItem<T>, ConfigError>;
}

impl <T: FromStr + PartialOrd + fmt::Display> Range<T> for Result<TypedItem<T>, ConfigError> {
	/// Makes sure that the configuration value is at least the given value.
	///
	/// Uses the
	/// [`PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html)
	/// trait to make sure the configured value is less or equal the given
	/// value.
	///
	/// ## Example
	///
	/// ```should_panic
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::validators::Range;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "4", "source info");
	/// conf.add_source(defaults);
	///
	/// // This will panic because the value 4 is less than 5.
	/// let value: i32 = conf.get(ConfPath::from(&["myitem"])).min(5).value().unwrap();
	/// ```
	fn min(self, minimum: T) -> Result<TypedItem<T>, ConfigError> {
		self?.filter(|v| if *v < minimum {
				Err(Box::new(ValidatorError::BelowMinimum(format!("{}", minimum))))
			} else {
				Ok(())
			}
		)
	}

	/// Makes sure that the configuration value is at most the given value.
	///
	/// Uses the
	/// [`PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html)
	/// trait to make sure the configured value is greater or equal to the given
	/// value.
	///
	/// ## Example
	///
	/// ```should_panic
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::validators::Range;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "10", "source info");
	/// conf.add_source(defaults);
	///
	/// // This will panic because the value 10 is more than 5.
	/// let value: i32 = conf.get(ConfPath::from(&["myitem"])).max(5).value().unwrap();
	/// ```
	fn max(self, maximum: T) -> Result<TypedItem<T>, ConfigError> {
		self?.filter(|v| if *v > maximum {
				Err(Box::new(ValidatorError::AboveMaximum(format!("{}", maximum))))
			} else {
				Ok(())
			}
		)
	}

	/// Makes sure that the configuration value is within a specified range.
	///
	/// Uses an implementaiton of the
	/// [`range`](https://doc.rust-lang.org/std/ops/struct.Range.html) trait to
	/// check if the configured value is within range.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// # use justconfig::validators::Range;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// defaults.set(conf.root().push_all(&["myitem"]), "10", "source info");
	/// conf.add_source(defaults);
	///
	/// // This will succeed because 10 is between 5 and 20.
	/// let value: i32 = conf.get(ConfPath::from(&["myitem"])).in_range(5..=20).value().unwrap();
	/// ```
	fn in_range<R: RangeBounds<T>>(self, range: R) -> Result<TypedItem<T>, ConfigError> {
		self?.filter(|v| if range.contains(v) {
				Ok(())
			} else {
				Err(Box::new(ValidatorError::from_range(&range)))
			}
		)
	}
}

impl <T: FromStr + PartialOrd + fmt::Display> Range<T> for Result<StringItem, ConfigError> where T::Err: Error + 'static {
	fn min(self, minimum: T) -> Result<TypedItem<T>, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).min(minimum)
	}

	fn max(self, maximum: T) -> Result<TypedItem<T>, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).max(maximum)
	}

	fn in_range<R: RangeBounds<T>>(self, range: R) -> Result<TypedItem<T>, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).in_range(range)
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::Config;
	use crate::confpath::ConfPath;
	use crate::item::ValueExtractor;
	use crate::sources::defaults::Defaults;

	#[test]
	fn range_good() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["ten"]), "10", "10");
		d.set(c.root().push_all(["five"]), "5", "5");
		d.set(c.root().push_all(["zero"]), "0", "0");
		d.set(c.root().push_all(["neg_one"]), "-1", "-1");
		c.add_source(d);

		// Test min
		assert_eq!(c.get(ConfPath::from(&["ten"])).min(5).value().unwrap(), 10u32);
		assert_eq!(c.get(ConfPath::from(&["five"])).min(5).value().unwrap(), 5u32);
		assert_eq!(c.get(ConfPath::from(&["zero"])).min(0).value().unwrap(), 0u32);
		assert_eq!(c.get(ConfPath::from(&["neg_one"])).min(-1).value().unwrap(), -1i32);

		// Test max
		assert_eq!(c.get(ConfPath::from(&["ten"])).max(10).value().unwrap(), 10u32);
		assert_eq!(c.get(ConfPath::from(&["five"])).max(10).value().unwrap(), 5u32);
		assert_eq!(c.get(ConfPath::from(&["zero"])).max(0).value().unwrap(), 0u32);
		assert_eq!(c.get(ConfPath::from(&["neg_one"])).max(0).value().unwrap(), -1i32);

		// Test between
		assert_eq!(c.get(ConfPath::from(&["ten"])).in_range(0..=10).value().unwrap(), 10u32);
		assert_eq!(c.get(ConfPath::from(&["ten"])).in_range(10..11).value().unwrap(), 10u32);
		assert_eq!(c.get(ConfPath::from(&["five"])).in_range(0..=10).value().unwrap(), 5u32);
		assert_eq!(c.get(ConfPath::from(&["zero"])).in_range(-10..=10).value().unwrap(), 0i32);
		assert_eq!(c.get(ConfPath::from(&["neg_one"])).in_range(-1..=0).value().unwrap(), -1i32);
	}

	#[test]
	#[should_panic(expected = "BelowMinimum")]
	fn range_min_bad() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["ten"]), "10", "10");
		c.add_source(d);

		let _: u32 = c.get(ConfPath::from(&["ten"])).min(20).value().unwrap();
	}

	#[test]
	#[should_panic(expected = "AboveMaximum")]
	fn range_max_bad() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["ten"]), "10", "10");
		c.add_source(d);

		let _: u32 = c.get(ConfPath::from(&["ten"])).max(5).value().unwrap();
	}

	#[test]
	#[should_panic(expected = "NotInRange")]
	fn range_between_bad_lower() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["ten"]), "10", "10");
		c.add_source(d);

		let _: u32 = c.get(ConfPath::from(&["ten"])).in_range(20..30).value().unwrap();
	}

	#[test]
	#[should_panic(expected = "NotInRange")]
	fn range_between_bad_upper() {
		let mut c = Config::default();
		let mut d = Defaults::default();

		d.set(c.root().push_all(["ten"]), "10", "10");
		c.add_source(d);

		let _: u32 = c.get(ConfPath::from(&["ten"])).in_range(0..5).value().unwrap();
	}
}
