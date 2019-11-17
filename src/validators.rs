use crate::ItemDef;
use std::fmt;
use std::error::Error;
use std::str::FromStr;

#[derive(Debug)]
pub enum ValidatorError {
	Empty,
	BelowMinimum(String),
	AboveMaximum(String),
	NotBetween(String, String)
}

impl fmt::Display for ValidatorError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Empty => write!(f, "must not be empty."),
			Self::BelowMinimum(min) => write!(f, "must be at least {}.", min),
			Self::AboveMaximum(max) => write!(f, "must be at most {}.", max),
			Self::NotBetween(min, max) => write!(f, "must be at least {} and at most {}.", min, max)
		}
	}
}

impl Error for ValidatorError {
}

pub trait NotEmpty {
	fn not_empty(&mut self) -> &mut Self;
}

impl NotEmpty for ItemDef {
	fn not_empty(&mut self) -> &mut Self {
		self.add(Box::new(|value| if value.is_empty() { Err(Box::new(ValidatorError::Empty)) } else { Ok(()) } ));
		self
	}
}

pub trait Range<T: PartialOrd + FromStr + fmt::Display + 'static> where <T as std::str::FromStr>::Err: std::error::Error {
	fn min(&mut self, minimum: T) -> &mut Self;
	fn max(&mut self, maximum: T) -> &mut Self;
	fn between(&mut self, minimum: T, maximum: T) -> &mut Self;
	fn positive(&mut self) -> &mut Self;
	fn negative(&mut self) -> &mut Self;
}

impl <T: PartialOrd + FromStr + fmt::Display + 'static> Range<T> for ItemDef where <T as std::str::FromStr>::Err: std::error::Error {
	fn min(&mut self, minimum: T) -> &mut Self {
		self.add(Box::new(move |value| if T::from_str(value)? < minimum { Err(Box::new(ValidatorError::BelowMinimum(format!("{}", minimum)))) } else { Ok(()) } ));
		self
	}

	fn max(&mut self, maximum: T) -> &mut Self {
		self.add(Box::new(move |value| if T::from_str(value)? > maximum { Err(Box::new(ValidatorError::AboveMaximum(format!("{}", maximum)))) } else { Ok(()) } ));
		self
	}

	fn between(&mut self, minimum: T, maximum: T) -> &mut Self {
		self.add(Box::new(move |value| { let v = T::from_str(value)?; if v < minimum || v > maximum { Err(Box::new(ValidatorError::NotBetween(format!("{}", minimum), format!("{}", maximum)))) } else { Ok(()) } } ));
		self
	}

	fn positive(&mut self) -> &mut Self {
		self
	}

	fn negative(&mut self) -> &mut Self {
		self
	}

}