//! Structures for representing configuration items and values.
//!
//! This basic `Item` structure is used to create the two fundamental types
//! of just-config:
//!
//! - [`StringItem`]
//! - [`TypedItem`]
//!
//! The configuration pipeline uses the two types of configuration items at
//! different stages. The configuration pipeline looks like the following:
//!
//! ```text
//! +--------+   +------------+   +------------+   +----------------+
//! | source +-->| processors +-->| validators |-->| ValueExtractor |
//! +--------+   +------------+   +------------+   +----------------+
//! ```
//!
//! To make this more transparent take the following example:
//!
//! ```rust
//! # use justconfig::Config;
//! # use justconfig::ConfPath;
//! # use justconfig::item::ValueExtractor;
//! # use justconfig::sources::defaults::Defaults;
//! # use justconfig::processors::Trim;
//! # use justconfig::validators::Range;
//! #
//! # let mut conf = Config::default();
//! # let mut defaults = Defaults::default();
//! # defaults.set(conf.root().push_all(&["myvalue"]), "3", "source info");
//! # conf.add_source(defaults);
//! #
//! let myvalue: u32 = conf.get(ConfPath::from(&["myvalue"])).trim().max(5).value().expect("myvalue not found");
//! ```
//!
//! The first part `conf.get` searches all registered configuration sources and
//! returns a `Result<StringItem, ConfigError>`.
//!
//! The following `trim()` method is a processor. Processors operate on the
//! string value of the configuration item and manipulate the string without
//! knowing anything about the meaning of the string.
//!
//! The next call is `max(5)`. This is a validator. Validators need to know more
//! about the meaning of the string value. Therefore the first call of validator
//! converts the `Result<StringItem, ConfigError>` into a
//! `Result<TypedItem<T>, ConfigError>`. To make this conversion possible, T must
//! implement the `FromStr` trait.
//!
//! This conversion is also responsible for the restriction, that all processors
//! have to be placed before the validators within the pipeline.
//!
//! The last call `value()` is implemented via the
//! [`ValueExtractor`] trait.
//! The `ValueExtractor` can (like [`validators`](crate::validators)) be called on
//! `Result<`[`StringItem`]`, `[`ConfigError`]`>` or `Result<`[`TypedItem<T>`]`, `[`ConfigError`]`>`. It
//! extracts the value from the pipeline and returns it to the caller. There are
//! multiple methods implemented for the [`ValueExtractor`] trait to be able to
//! return different kinds of values:
//!
//! * Optional values
//! * Multiple values
//! * Single, mandatory values
use crate::confpath::ConfPath;
use crate::error::ConfigError;
use std::str::FromStr;
use std::rc::Rc;
use std::convert::TryInto;
use std::error::Error;
use std::ops::RangeBounds;

/// Trait implemented by source location structs provided by data sources.
///
/// This trait is used to provide the source of a configuration entry, for
/// example, for use in error messages.
pub trait SourceLocation : std::fmt::Display + std::fmt::Debug {}

/// Structure representing a configuration value.
///
/// Any configuration item can have multiple configuration values.
///
/// Every configuration value is linked to its source. Every configuration source
/// implements a struct that implements the `SourceLocation` trait. The source
/// location is used to supply information to the user where the configuration
/// value is coming from.
///
/// See [`Item`](crate::item) for more Information.
pub struct Value<T> {
	value: T,
	source: Rc<dyn SourceLocation>
}

impl <T> Value<T> {
	/// Create a new configuration value.
	///
	/// Configuration values are normally created to be included into configuration
	/// [`Item`](crate::item)s.
	pub fn new(value: T, source: Rc<dyn SourceLocation>) -> Rc<Self> {
		Rc::new(Self {
			value,
			source
		})
	}

	/// Returns the source of this configuration value.
	pub fn source(&self) -> Rc<dyn SourceLocation>{
		self.source.clone()
	}
}

#[derive(Clone)]
struct Item<T> {
	key: ConfPath,
	values: Vec<Rc<Value<T>>>
}

/// Newtype for Items while they are passed though the processors of the config
/// pipeline.
///
/// `StringItem` implements some additional methods that are useful while a new
/// `Item` is created within a config source.
/// See [`Source`](crate::Source) for more information.
///
/// For more information about processors and validators see
/// [`Item`](crate::item).
#[derive(Clone)]
pub struct StringItem(Item<String>);

impl StringItem {
	pub(crate) fn new(key: ConfPath) -> Self {
		Self (Item {
			key,
			values: Vec::with_capacity(1)
		})
	}

	pub(crate) fn from(key: ConfPath, values: &[Rc<Value<String>>]) -> Self {
		Self (Item {
			key,
			values: Vec::from(values)
		})
	}

	pub(crate) fn push(&mut self, new_value: Rc<Value<String>>) {
		self.0.values.push(new_value);
	}

	pub(crate) fn clear(&mut self) -> &mut Self {
		self.0.values.clear();
		self
	}
}

/// Newtype for Items while they are passed though the validators of the config
/// pipeline and to the [`ValueExtractor`].
///
/// See [`Item`](crate::item) for more Information.
#[derive(Clone)]
pub struct TypedItem<T: FromStr>(Item<T>);

impl <T: FromStr> TypedItem<T> {
	pub(crate) fn new(key: ConfPath, values: Vec<Rc<Value<T>>>) -> Self {
		Self(Item {
			key,
			values
		})
	}
}

impl <T: FromStr> TypedItem<T> {
	pub fn filter(self, filter: impl Fn(&T) -> Result<(), Box<dyn Error>>) -> Result<Self, ConfigError> {
		for v in self.0.values.iter() {
			filter(&v.value).map_err(|e| ConfigError::ValueError(e, v.source.clone()))?;
		}

		Ok(self)
	}
}

pub enum MapAction {
	Keep,
	Replace(Vec<String>),
	Drop,
	Fail(Box<dyn Error>)
}

impl StringItem {
	pub fn map(self, mapper: impl Fn(&String) -> MapAction) -> Result<Self, ConfigError> {
		let mut mapped_item = StringItem::new(self.0.key);

		for v in self.0.values.into_iter() {
			match mapper(&v.value) {
				MapAction::Keep => mapped_item.push(v),
				MapAction::Replace(new_values_list) => for value in new_values_list.into_iter().map(|mapped_v| Value::new(mapped_v, v.source.clone())) { mapped_item.push(value); },
				MapAction::Drop => (),
				MapAction::Fail(error) => return Err(ConfigError::ValueError(error, v.source.clone()))
			}
		}

		Ok(mapped_item)
	}
}

impl <T: FromStr> TryInto<TypedItem<T>> for Result<StringItem, ConfigError> where T::Err: Error + 'static {
	type Error = ConfigError;

	fn try_into(self) -> Result<TypedItem<T>, ConfigError> {
		let s = self?;

		// Iterate all String-Values...
		let typed_values: Result<Vec<Rc<Value<T>>>, ConfigError> = s.0.values.into_iter().map(|v|
			// ...and convert them to T...
			v.value.parse::<T>().map(|nv|
				Value::new(nv, v.source.clone())
			)
			// ...if an error occures while converting, map it to a ConfigError...
			.map_err(|e| ConfigError::from_error(e, v.source.clone()))
		)
		//.. and collect everything. If there is one Result::Err this will lead to an err on the final collection
		.collect();

		Ok(TypedItem::new(s.0.key, typed_values?))
	}
}

/// Trait implemented for `TypedItem` and `StringItem` to allow retrieval of the
/// stored config value.
///
/// This Trait is implemented for `Result<TypedItem<T>, ConfigError>` and
/// `Result<StringItem, ConfigError>`. This makes sure that the methods can be
/// called on the raw `StringItems` and on the `TypedItems` returned by
/// [validators](crate::validators).
///
/// The Implementation for `StringItem` will do the same conversion that is
/// normally done when calling a validator.
pub trait ValueExtractor<T: FromStr> {
	/// Returns a configuration value if it exists or ´None´ otherwise.
	///
	/// An error is only returned if one of the following occures:
	/// * The value of the configuration item could not be converted into the
	///   required data type.
	/// * A processor or validator returned an error.
	/// * There is more that one value available.
	///
	/// This method should be used to return optional configuration values.
	/// A default value can be provided by using `unwrap_or`.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// # defaults.set(conf.root().push_all(&["myvalue"]), "3", "source info");
	/// # conf.add_source(defaults);
	/// #
	/// let myvalue: u32 = conf.get(ConfPath::from(&["myvalue"])).try_value().expect("Error").unwrap_or(0);
	/// ```
	fn try_value(self) -> Result<Option<T>, ConfigError>;

	/// Returns a configuration value or raises an error if it does not exists.
	///
	/// This method works like [`try_value()`](Self::try_value) but returns an error if the
	/// configuration item does not exist.
	///
	/// This method should be used to return mandatory configuration values that
	/// should result in an error if they are not found.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// # defaults.set(conf.root().push_all(&["myvalue"]), "3", "source info");
	/// # conf.add_source(defaults);
	/// #
	/// let myvalue: u32 = conf.get(ConfPath::from(&["myvalue"])).value().expect("Error or not found");
	/// ```
	fn value(self) -> Result<T, ConfigError>;

	/// Returns all configuration values for a configuration item.
	///
	/// This is the only method that allows more than one configuration value
	/// to exist. Use this method to read multi value items. If the
	/// configuration item does not exist, an empty array is returned.
	///
	/// The method accepts a range to specify how many configuration values can
	/// be set for this item. If any number of configuration values are
	/// acceptible, just specify `..`. If the number of values should be
	/// limited, specify a range.
	///
	/// ## Example
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// # defaults.set(conf.root().push_all(&["myvalue"]), "3", "source info");
	/// # conf.add_source(defaults);
	/// #
	/// let myvalue: Vec<u32> = conf.get(ConfPath::from(&["myvalue"])).values(..).expect("Error");
	/// ```
	///
	/// If there must be at least one instance of `myvalue` specify a range of
	/// `1..`:
	///
	/// ```rust
	/// # use justconfig::Config;
	/// # use justconfig::ConfPath;
	/// # use justconfig::item::ValueExtractor;
	/// # use justconfig::sources::defaults::Defaults;
	/// #
	/// # let mut conf = Config::default();
	/// # let mut defaults = Defaults::default();
	/// # defaults.set(conf.root().push_all(&["myvalue"]), "3", "source info");
	/// # conf.add_source(defaults);
	/// #
	/// let myvalue: Vec<u32> = conf.get(ConfPath::from(&["myvalue"])).values(1..).expect("Error");
	/// ```
	///
	/// If the number of values should be limited to at most 3 values the range
	/// must be `..=3`.
	///
	fn values<R: RangeBounds<usize>>(self, range: R) -> Result<Vec<T>, ConfigError>;
}

#[allow(clippy::unnecessary_unwrap)] // Until https://github.com/rust-lang/rfcs/pull/2497 gets implemented
fn values_out_of_range<T: FromStr, R: RangeBounds<usize>>(mut item: TypedItem<T>, range: R) -> Result<Vec<T>, ConfigError> {
	let num_items = item.0.values.len();

	if range.contains(&num_items) {
		item.0.values.drain(..).map(|r| Rc::try_unwrap(r).map(|v| v.value).map_err(|_| ConfigError::MultipleReferences)).collect()
	} else {
		// The number of items is not part of the range. Check if the upper or lower bound
		// was violated.
		// The lower limit is inclusive, the upper is exlusive. This is done to make sure
		// we do not underflow the unsigned value.
		let lower_limit_inc = match range.start_bound() {
			std::ops::Bound::Included(min) => Some(*min),
			std::ops::Bound::Excluded(min) => Some(*min + 1),
			std::ops::Bound::Unbounded => None
		};

		let upper_limit_excl = match range.end_bound() {
			std::ops::Bound::Included(max) => Some(*max + 1),
			std::ops::Bound::Excluded(max) => Some(*max),
			std::ops::Bound::Unbounded => None
		};

		if lower_limit_inc.is_some() && (num_items < lower_limit_inc.unwrap()) {
			// Lower bound violated
			Err(ConfigError::NotEnoughValues(lower_limit_inc.unwrap(), item.0.key))
		} else if upper_limit_excl.is_some() && (num_items >= upper_limit_excl.unwrap()) {
			// Upper bound violated
			let first_surplus_index = upper_limit_excl.unwrap().saturating_sub(1);
			let surplus_sources = item.0.values.drain(first_surplus_index..).map(|r| Rc::try_unwrap(r).map(|v| v.source).map_err(|_| ConfigError::MultipleReferences)).collect::<Result<Vec<Rc<dyn SourceLocation>>, ConfigError>>()?;

			Err(ConfigError::TooManyValues(first_surplus_index, item.0.key, surplus_sources))
		} else {
			unreachable!("This is not possible because we checked that num_items is not contained in range.");
		}
	}
}

impl <T: FromStr> ValueExtractor<T> for Result<TypedItem<T>, ConfigError> {
	fn try_value(self) -> Result<Option<T>, ConfigError> {
		match self.value() {
			Ok(value) => Ok(Some(value)),
			Err(ConfigError::ValueNotFound(_)) => Ok(None),
			Err(error) => Err(error)
		}
	}

	fn value(self) -> Result<T, ConfigError> {
		let mut ci = self?.0;

		match ci.values.len() {
			0 => Err(ConfigError::ValueNotFound(ci.key)),
			1 => Rc::try_unwrap(ci.values.pop().unwrap()).map(|v| v.value).map_err(|_| ConfigError::MultipleReferences),
			_ => Err(ConfigError::TooManyValues(1, ci.key, ci.values.iter().map(|v| v.source()).collect()))
		}
	}

	fn values<R: RangeBounds<usize>>(self, range: R) -> Result<Vec<T>, ConfigError> {
		// This match converts a ValueNotFound error into an empty vector.
		// This makes sure that an empty value-vectors is equvalent with an ValueNotFound error for all purposes.
		match self {
			Ok(item) => values_out_of_range(item, range),
			Err(ConfigError::ValueNotFound(key)) => values_out_of_range(TypedItem::<T>::new(key, Vec::default()), range), // Create an empty TypedItem to allow handling the config range correctly
			Err(error) => Err(error)
		}
	}
}

impl <T: FromStr> ValueExtractor<T> for Result<StringItem, ConfigError> where T::Err: Error + 'static {
	fn try_value(self) -> Result<Option<T>, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).try_value()
	}

	fn value(self) -> Result<T, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).value()
	}

	fn values<R: RangeBounds<usize>>(self, range: R) -> Result<Vec<T>, ConfigError> {
		(self.try_into() as Result<TypedItem<T>, ConfigError>).values(range)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Config;
	use crate::sources::defaults::Defaults;
	use crate::error::ConfigError;

	fn prepare_test_config() -> Config {
		let mut c = Config::default();

		let mut defaults = Defaults::default();
		defaults.empty(c.root().push_all(["no_value"]));
		defaults.set(c.root().push_all(["one_value"]), "one_value", "1.1");
		defaults.put(c.root().push_all(["two_values"]), "two_values", "2.1");
		defaults.put(c.root().push_all(["two_values"]), "two_values", "2.2");
		c.add_source(defaults);

		c
	}

	#[test]
	fn value_no_value() {
		let c = prepare_test_config();

		assert_eq!(format!("{}", (c.get(c.root().push_all(["no_value"])).value() as Result<String, ConfigError>).unwrap_err()), "Missing value for config key 'no_value'.");
	}

	#[test]
	fn value_one_value() {
		let c = prepare_test_config();

		assert_eq!((c.get(c.root().push_all(["one_value"])).value() as Result<String, ConfigError>).unwrap(), "one_value");
	}

	#[test]
	fn value_two_values() {
		let c = prepare_test_config();

		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).value() as Result<String, ConfigError>).unwrap_err()), "More than 1 value found for key two_values@['default from 2.1', 'default from 2.2']");
	}

	#[test]
	fn try_value_no_value() {
		let c = prepare_test_config();

		assert!((c.get(c.root().push_all(["no_value"])).try_value() as Result<Option<String>, ConfigError>).unwrap().is_none());
	}

	#[test]
	fn try_value_one_value() {
		let c = prepare_test_config();

		assert_eq!((c.get(c.root().push_all(["one_value"])).try_value() as Result<Option<String>, ConfigError>).unwrap().unwrap(), "one_value");
	}

	#[test]
	fn try_value_two_values() {
		let c = prepare_test_config();

		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).try_value() as Result<Option<String>, ConfigError>).unwrap_err()), "More than 1 value found for key two_values@['default from 2.1', 'default from 2.2']");
	}

	#[test]
	fn values_no_value() {
		let c = prepare_test_config();

		let values: Vec<String> = c.get(c.root().push_all(["no_value"])).values(..).unwrap();
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn values_one_value() {
		let c = prepare_test_config();

		let mut values: Vec<String> = c.get(c.root().push_all(["one_value"])).values(..).unwrap();
		assert_eq!(values.len(), 1);
		assert_eq!(values.pop().unwrap(), "one_value");
	}

	#[test]
	fn values_two_values() {
		let c = prepare_test_config();

		let mut values: Vec<String> = c.get(c.root().push_all(["two_values"])).values(..).unwrap();
		assert_eq!(values.len(), 2);
		assert_eq!(values.pop().unwrap(), "two_values");
		assert_eq!(values.pop().unwrap(), "two_values");
	}

	#[test]
	fn range_lower_limit() {
		let c = prepare_test_config();

		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).values(3..) as Result<Vec<String>, ConfigError>).unwrap_err()), "Key \'two_values\' must have at least 3 values.");
		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).values(4..5) as Result<Vec<String>, ConfigError>).unwrap_err()), "Key \'two_values\' must have at least 4 values.");
		assert_eq!(format!("{}", (c.get(c.root().push_all(["no_value"])).values(1..) as Result<Vec<String>, ConfigError>).unwrap_err()), "Key \'no_value\' must have at least 1 values.");
		assert_eq!(format!("{}", (c.get(c.root().push_all(["unkown_key"])).values(1..) as Result<Vec<String>, ConfigError>).unwrap_err()), "Key \'unkown_key\' must have at least 1 values.");
	}

	#[test]
	fn range_upper_limit() {
		let c = prepare_test_config();

		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).values(..=1) as Result<Vec<String>, ConfigError>).unwrap_err()), "More than 1 value found for key two_values@['default from 2.2']");
		assert_eq!(format!("{}", (c.get(c.root().push_all(["two_values"])).values(1..=1) as Result<Vec<String>, ConfigError>).unwrap_err()), "More than 1 value found for key two_values@['default from 2.2']");
	}

	#[test]
	fn range_ok() {
		let c = prepare_test_config();

		let mut values: Vec<String> = c.get(c.root().push_all(["two_values"])).values(1..=2).unwrap();
		assert_eq!(values.len(), 2);
		assert_eq!(values.pop().unwrap(), "two_values");
		assert_eq!(values.pop().unwrap(), "two_values");

		let mut values: Vec<String> = c.get(c.root().push_all(["two_values"])).values(2..3).unwrap();
		assert_eq!(values.len(), 2);
		assert_eq!(values.pop().unwrap(), "two_values");
		assert_eq!(values.pop().unwrap(), "two_values");

		let mut values: Vec<String> = c.get(c.root().push_all(["two_values"])).values(0..10).unwrap();
		assert_eq!(values.len(), 2);
		assert_eq!(values.pop().unwrap(), "two_values");
		assert_eq!(values.pop().unwrap(), "two_values");

		let mut values: Vec<String> = c.get(c.root().push_all(["one_value"])).values(..3).unwrap();
		assert_eq!(values.len(), 1);
		assert_eq!(values.pop().unwrap(), "one_value");

		let mut values: Vec<String> = c.get(c.root().push_all(["one_value"])).values(1..).unwrap();
		assert_eq!(values.len(), 1);
		assert_eq!(values.pop().unwrap(), "one_value");

		let values: Vec<String> = c.get(c.root().push_all(["unkown_key"])).values(..).unwrap();
		assert_eq!(values.len(), 0);

		let values: Vec<String> = c.get(c.root().push_all(["unkown_key"])).values(..1).unwrap();
		assert_eq!(values.len(), 0);

		let values: Vec<String> = c.get(c.root().push_all(["unkown_key"])).values(..=1).unwrap();
		assert_eq!(values.len(), 0);

		let values: Vec<String> = c.get(c.root().push_all(["unkown_key"])).values(..=0).unwrap();
		assert_eq!(values.len(), 0);
	}
}
