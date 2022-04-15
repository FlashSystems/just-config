![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)
[![Current Version](https://img.shields.io/crates/v/justconfig)](https://crates.io/crates/justconfig)
[![Docs.rs](https://docs.rs/justconfig/badge.svg)](https://docs.rs/justconfig)
![License Apache 2.0](https://img.shields.io/crates/l/justconfig)

# Config Library for Rust

Just-config is a configuration library for rust. It strives for the old Unix mantra "Do one thing and to it well". It's just build to read configuration values from different sources and fuse them into an easy to handle configuration source. Its primary purpose is to be used by a configuration class and populate the different, typed configuration values.

Out of the box it features the following configuration sources:

* Static (fallbacks, command line)
* Environment variables
* Configuration file

It has built in validation support and can accept multiple configuration values per key. It even can limit the number of configuration values that are acceptable for a given configuration key.

Writing your own configuration sources (for example for etcd) is really easy. You only have to implement the `get` method of the [`Source` trait](https://docs.rs/justconfig/latest/justconfig/source/trait.Source.html).

If you just want to use this library, open the documentation, look at the examples and descriptions and start using it by adding the following to the `[dependencies]` section of your `Cargo.toml`:

```toml
justconfig = "1.0"
```

## Basic example

A a little teaser here is the basic example copied from the
[documentation](https://docs.rs/justconfig).

```rust
use justconfig::Config;
use justconfig::ConfPath;
use justconfig::sources::text::ConfigText;
use justconfig::sources::env::Env;
use justconfig::sources::defaults::Defaults;
use justconfig::processors::Explode;
use justconfig::validators::Range;
use justconfig::item::ValueExtractor;
use std::ffi::OsStr;
use std::fs::File;
let mut conf = Config::default();
// Allow some environment variables to override configuration values read
// from the configuration file.
let config_env = Env::new(&[
  (ConfPath::from(&["searchPath"]), OsStr::new("SEARCH_PATH")),
]);
// Open the configuration file
let config_file = File::open("myconfig.conf").expect("Could not open config file.");
conf.add_source(ConfigText::new(config_file, "myconfig.conf").expect("Loading configuration file failed."));
// Read the value `num_frobs` from the configuration file.
// Do not allow to use more than 10 frobs.
let num_frobs: i32 = conf.get(conf.root().push("num_frobs")).max(10).value()?;
// Read a list of tags from the configuration file.
let tag_list: Vec<String> = conf.get(conf.root().push("tags")).values(..)?;
// Read the paths from the config file and allow it to be overriden by
// the environment variable. We split everything at `:` to allow passing
// multiple paths using an environment variable. When read from the config
// file, multiple values can be set without using the `:` delimiter.
// Passing 1.. to values() makes sure at least one search path is set.
let search_paths: Vec<String> = conf.get(conf.root().push("searchPath")).explode(':').values(1..)?;
```

## Changelog

* Version 0.8.0\
  Initial Release

* Version 0.8.1\
  Add some more examples

* Version 0.9.0\
  **Breaking change**: Added range syntax for configuration values and range validation. All occurrences of `values()` and `between()` must be updated. The error handling for validation errors of the `between`-validator has changes as well.

* Version 0.9.1\
  Added the `stack_config` function to the `text` source module. This function makes merging
  configuration files from multiple source paths easier.

* Version 0.9.2\
  Updated documentation to use intra-doc-links.

* Version 1.0.0\
  Fixed that non existent configuration keys satisfied a `1..` range limit. Now this is correctly detected as an error.\
  Updates to documentation to mention `stack_config` on the library page.

* Version 1.0.1\
  Cosmetic Code changes to fix some clippy warnings.

## Design rational

If you are interested about the rationale behind the design of this library (and can stand some highly opinionated reasoning) you can read on.

### No data types

I don't think that data types in configuration files are a good idea. What do I mean by "data types"? TOML for example distinguishes strings, numbers, dates, etc. They are represented differently within the configuration file. Let's assume you've got a configuration file that reads `cache_timeout=42`. That's ok, but now the next version of your program should allow the user to disable the cache. You think about it and decide (like many have done before) to use the value 0 as a magic value for disabling the cache. That's fine for now. But after a few versions, you want to add infinite cache timeout. You can't just use `"infinite"` or some other string, because all old configuration files only have the number in there. You can allow both, strings and numbers but that increases the complexity and it does something else: It makes the configuration file much harder to understand. The `infinite` value is not a string. It's a special constant (a literal) and putting it between quotation marks conveys the completely wrong message. A string is an arbitrary sequence of characters that can be chosen by the user. Not a single, constant value. By having different data types within your configuration file you've created a restriction for you as a developer and/or a maintenance burden for the user.

I think a better solution is to keep the data type out of the configuration format. `cache_timeout=42` is a valid value and `cache_timeout=infinite` is also valid. Go ahead and add `cache_timeout="twenty minutes"`. It's up to the application to determine the meaning of the value part. If you want to put an exclamation mark in front of your constants: Just do it. The configuration library should not impose any restrictions on you.

### Line continuation

Many configuration file format use line continuation on the line that is continued. For example TOML allows to continue a line by ending it with a backslash. This approach has some drawbacks: When writing a multi line value, look-ahead is needed to determine if the current line has to end with a continuation character. This makes automatic writing a multi line value more complicated that it should be. Event for human users this is not very convenient. You have to go up one line and append the continuation character to the previous line to continue it. The LDIF format uses a better solution: Marking continuation on the continuing line. This prevents a security problem as well:

Imagine the following configuration file:

```ini
multiline=line1 \
line2 \
line3
critical_value=secure
```

Now you delete `line3` but miss to delete the continuation character on `line2`. Now the security `critical_value` is just part of `multiline`. If the value is security critical but optional you just created a security vulnerability.

### Leading white-space

Leading white-space should be left to the user. Making them significant (like in YAML) creates two types of annoyances for the user:

1) The user should be able to indent its configuration files in any way he finds reasonable. Even unreasonable ways should not be a problem.
2) Distinguishing the number of white spaces and tabs on a non-working server, in a hurry with not more than basic `vi` is hard and many administrators will get it wrong at least one time.

And from a security standpoint: A missing white space, that moves your critical configuration value into a section where it does nothing, is a problem as well.

### No write support

Configuration files are for the user. He's the only one that should write to a configuration file. Don't mess with it. The configuration file might be part of an automated deployment workflow, that will get upset if you decide to change the contents of the configuration file. If you want to help your user to create the first configuration file, supply an example. If your application has to write to the file, it's not a configuration file anymore. It's a database. Just use a different library and format for that (sqlite, yaml, xml).

There is one exception to this rule. If you're writing a configuration management or deployment solution, you have to write configuration files. But there are good templating engines out there that will get the job done.

### No deserialization

Configuration files are parsed, not deserialized. Serialization is the process of turning a complex data structure into a _series_ of tokens (mostly bytes) and later reconstructing the data structure from these tokens later. Configuration information is not a data structure to begin with. Trying to turn it into one turns often out to be a problem in the end. At first serialization libraries look like a good solution. But as your configuration information grows and the number of configuration sources rises it becomes more and more complex.

Different sources have different capabilities in expressing the configuration information. If the information is coming from an environment variable using a separator character for multiple values might be a good choice. For a configuration file simply using multiple entries with the same key might be more intuitive.

Most serialization libraries are not build to give you in depth control over the format you want to parse. And soon you're developing your own parser on top of the chosen library.

If you're searching for a serialization library I recommend you to take a look at [serde](https://crates.io/crates/serde).

### No unsafe code

A configuration file parser by definition is parsing untrusted information. You should not make things worse by using `unsafe` code. Not using `unsafe` is no guarantee for safe code, but using it drops many of the guarantees rust gives you. This should only be done with a good reason. And parsing a text file or environment variable is no good reason.

### No dependencies

And one last thing: **No dependencies**. This is a configuration file parser, not an application framework. I think pulling in dependencies and sub dependencies into a project, that only wants to parse some configuration information is rather rude and increases the maintenance burden for the consumer of the configuration library. Every dependency can have security issues that you must track and force updates on your product because of that. Sure, there are libraries that are totally worth it. But I think a configuration library should not do that. It should be simple enough to work without using any dependencies.
