
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

## TODO

An incomplete list:

* CI/CD, to deliver on the promise of being faster to install than Gollum.
* Put some chrome around the rendered markdown pages.
* Also get some nice RSS.
* Maybe a list of files when viewing directories, like GitHub does
* Editing support
* Some crate with mime types, so I don't have to hard code everything myself
  for pictures and other media.
* Figure out how compatible with Gollum we should be.
* Eventually figure out an API. Consider the possibility of being a drop-in
  replacement for Gollum, if that makes sense.
* If the sites hosted by this are ever to be exposed to the public, consider
  security issues:
  * Path traversal
  * XSS in wiki page contents and other places.
  * CSRF
  * Sandboxing, using things like Capsicum or `pledge(2)`.
* Consider performance. Specifically the page responses are currently rendered
  into a String. Memory usage and copying could be reduced by writing directly
  into the HTTP connection. The templating library and markdown already support
  the `std::io::Write` trait.
* Some sort of filesystem abstraction. Currently we read directly from the
  filesystem, but Gollum can read directly from bare Git repos. It would be nice
  to support both.
* Consider a fancier web app framework than Hyper.
* Consider supporting markup formats other than Markdown.

## Why Rust, please tell me more about why you love Rust

Rust makes it easy to ship compiled executables that run without much fuss.
As for why not some other language also shares this capability (Go or C#),
I just want to get more experience working with Rust.
