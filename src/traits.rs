use std::iter::Iterator;
use std::error::Error;

pub type Validator = dyn Fn (&str) -> Result<(), Box<dyn Error>>;

pub trait Source {
	fn get_config(self) -> dyn Iterator<Item = (String, String)>;
}