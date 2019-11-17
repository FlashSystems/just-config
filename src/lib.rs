use std::collections::HashMap;
use std::collections::hash_map::Entry;

mod traits;
use traits::{Source, Validator};

mod error;
use error::ConfigError;

mod validators;

pub struct ItemDef {
	validators: Vec<Box<Validator>>
}

impl ItemDef {
	pub fn new() -> Self {
		Self {
			validators: Vec::new()
		}
	}

	pub fn add(&mut self, validator: Box<Validator>) -> &mut Self {
		self.validators.push(validator);
		self
	}
}


struct Item {
	definition: ItemDef,
	values: Vec<String>
}

impl Item {
	pub fn new() -> Self {
		Self {
			definition: ItemDef::new(),
			values: Vec::with_capacity(1)
		}
	}
}

pub struct Config {
	items: HashMap<String, Item>,
	config_source: Vec<Box<dyn Source>>
}

impl Config {
	/// Create a new configuration store.
	pub fn new() -> Self {
		Self {
			items: HashMap::new(),
			config_source: Vec::new()
		}
	}

	pub fn add_source(&mut self, source: Box<dyn Source>) {
		self.config_source.push(source);
	}

	pub fn define(&mut self, key: &str) -> &mut ItemDef {
		let entry = self.items.entry(String::from(key));

		match entry {
			Entry::Occupied(_) => panic!("Configuration key {} already defined.", key),
			Entry::Vacant(vacant) => &mut vacant.insert(Item::new()).definition
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use validators::*;

	struct TestSource {
		items: Vec<(String, String)>
	}

	impl TestSource {
		fn new() -> Self {
			Self {
				items: vec![("empty".to_owned(), "".to_owned()), ("text".to_owned(), "this is text".to_owned()), ("ten".to_owned(), "10".to_owned())]
			}
		}
	}

	impl Source for TestSource {
		fn get_config(self) -> dyn Iterator<Item = (String, String)> {
			Box::new(self.items.into_iter())
		}
	}

	#[test]
	fn it_works() {
		let mut c = Config::new();

		c.define("text").not_empty();
		c.define("ten").between(5, 15);
		c.define("ten").min(5).max(15);
	}
}
