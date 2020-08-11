//! General error enums.
use crate::item::SourceLocation;
use crate::confpath::ConfPath;
use std::rc::Rc;

/// Enum used to return errors from the pipeline.
#[derive(Debug)]
pub enum ConfigError {
	/// A required configuration value was not found.
	ValueNotFound(ConfPath),
	/// If [`value()`](../item/trait.ValueExtractor.html#tymethod.value) is
	/// called on an item that has more that one value or if the number of
	/// values on a call to 
	/// [`values()`](../item/trait.ValueExtractor.html#tymethod.values) is out
	/// of range this error is returned. The location of the error
	/// is represented by an instance of a struct implementing the
	/// [`SourceLocation'](../item/trait.SourceLocation.html) trait. The first
	/// parameter contains the maximum number of values this configuration item
	/// can have.
	TooManyValues(usize, ConfPath, Vec<Rc<dyn SourceLocation>>),
	/// If [`values()`](../item/trait.ValueExtractor.html#tymethod.values) is
	/// called with a range restricting the valid number of values and there are
	/// not enough values this error is returned. The first parameter is
	/// the minimum number of values that this configuration item must contain
	/// to be valid.
	NotEnoughValues(usize, ConfPath),
	/// This error is returned if the conversion of the string value into a
	/// typed value failed or if a processor/validator returns an error.
	/// The location of the error is represented by an instance of a struct
	/// implementing the [`SourceLocation'](../item/trait.SourceLocation.html)
	/// trait.
	ValueError(Box<dyn std::error::Error>, Rc<dyn SourceLocation>),
	/// Is returned if the pipeline is not linear. This should never happen if
	/// this library is used correctly.
	MultipleReferences
}

fn too_many_values_formater(f: &mut std::fmt::Formatter, max_num: usize, key: &ConfPath, source_locations: &[Rc<dyn SourceLocation>]) -> std::fmt::Result {
	write!(f, "More than {} value found for key {}@[", max_num, key)?;
	for (i, source_location) in source_locations.iter().enumerate() {
		if i > 0 {
			write!(f, ", ")?;
		}

		write!(f, "'{}'", source_location)?;
	};
	write!(f, "]")
}

impl std::fmt::Display for ConfigError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::ValueNotFound(key) => write!(f, "Missing value for config key '{}'.", key),
			Self::TooManyValues(max_num, key, source_locations) => too_many_values_formater(f, *max_num, key, source_locations),
			Self::NotEnoughValues(min_num, key) => write!(f, "Key '{}' must have at least {} values.", key, min_num),
			Self::ValueError(error, source_location) => write!(f, "{}@'{}'", error, source_location),
			Self::MultipleReferences => write!(f, "Internal error. Multiple references to same config pipeline.")
		}
	}
}

impl std::error::Error for ConfigError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::ValueError(error, _) => Some(error.as_ref()),
			_ => None
		}
	}
}

impl ConfigError {
	pub fn from_error<E: std::error::Error + 'static>(error: E, source_location: Rc<dyn SourceLocation>) -> Self {
		ConfigError::ValueError(Box::from(error), source_location)
	}
}
