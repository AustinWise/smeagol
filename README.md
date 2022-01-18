
# Smeagol - locally hosted wiki

The aspiration goal of this project is to create a wiki software with these
properties:

* Comparable in functionality to and compatible with GitHub wikis. This includes:
  * Git is the backing store
  * Markdown is used to format pages
* Easy to run on your local computer.
* Really fast and easy to install.

Non goals include:

* Support for multiple users.

Phrased in another way, the goal of this project is to create something roughly
like [Gollum](https://github.com/gollum/gollum), but does not take half an hour
to `gem install` on a low-powered Chromebook.

## Why Rust, please tell me more about why you love Rust

Rust makes it easy to ship cross-compiled executables that run without much fuss.
As for why not some other language also shares this capability (Go or C#),
I just want to get more experience working with Rust.

## License

Licensed under the [MIT License](LICENSE).

Note that some elements, specifically aspects of the visual design,
have been copied from [Gollum](https://github.com/gollum/gollum).
