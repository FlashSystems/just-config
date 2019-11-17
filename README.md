# Eazy Config Library for Rust

## No data types

I don't think data types in configuration files are a good idea. What do I mean by "data types"? TOML for example distinguishes strings, numbers, dates, etc. as different data types. They are represented differently within the configuration file. Let's assume you've got a configuration file that reads `cache_timeout=42`. That's ok, but now the next version of your program allow the user to disable the cache. You think about it and decide (like many have done before) to use the value 0 as a magic value for disabling the cache. That's fine for now. But after a few versions, you wont do add infinite cache timeout. You can't use `"infinite"` or some other string, because all old configuration files only have the number in there. You can allow both, strings and numbers but that increases the complexity and it does something else. It makes the configuration file much harder to understand. The `infinite` value is not a string. It's a special constant (a literal) and putting it between quotation marks sends the completely wrong signal. A string is an arbitrary sequence of characters that can be chosen by the user. Not a single, constant value. By having different data types within your configuration file you've created a restriction for you as a developer and/or a maintenance burden for the user.

I think a better solution is to keep the data type out of the configuration format. `cache_timeout=42` is a valid value and `cache_timeout=infinite` is also valid. As well as `cache_timeout=disabled`. It's up to the application to determine the meaning of the value part.

## No write support

Configuration files are for the user. He's the only one that should write to a configuration file. Don't mess with it. The configuration file might be part of an automated deployment workflow that will get upset if you decide to change the contents of the configuration file. If you want to help your user to create the first configuration file, supply an example. If your application has to write to the file, it's not a configuration file anymore. It's a database. Just use a different library and format for that (sqlite, yaml, xml).

There is one exception to this rule. If you're writing a configuration management or deployment solution, you have to write configuration files. But there are good templating engines out there that will get the job done.

## No deserialization

