0.5.0
=====

* Add the ability to create a new page. Thanks @kachick! [#72](https://github.com/AustinWise/smeagol/pull/72)
* Show the version and Git hash at the bottom of the page. Thanks @kachick! [#73](https://github.com/AustinWise/smeagol/pull/73)
* Update version of Ubuntu used to build release and run CI. Thanks @kachick! [#74](https://github.com/AustinWise/smeagol/pull/74)
* Various dependency updates.

0.4.10 (2024-11-07)
=====

This release contains fixes for compiling Smeagol from source.

* Update the `time` crate to fix a build error with the latest Rust compiler.
* Update `tanvity` to fix build when building with the latest version of `zstd`. Thanks @kachick! [#66](https://github.com/AustinWise/smeagol/issues/66)

0.4.9 (2024-04-06)
=====

Use `/DEPENDENTLOADFLAG` to mitigate hypothetical DLL injection attacks on Windows.
This was attempted in the prior release, but CI did not pick up the flag.

0.4.7 (2024-04-05)
=====

* Implement support for transclusion (Thanks @therealbstern!) [#61](https://github.com/AustinWise/smeagol/issues/61)
* Update some dependencies based on CVEs that Dependabot flagged. These CVEs are unlikely to pose
  a risk to typical uses of Smeagol.

0.4.5 (2023-12-21)
=====
Update dependencies. Not functional changes intended.

0.4.4 (2023-04-07)
=====

* When navigating to a directory with no index file, instead show the overview page. This has a list
  of files and folders.
* The Overview button will show the files in the directory that contains the files you are
  currently viewing. Previously it would always show the root folder.

0.4.3 (2022-10-08)
=====

* Fix lack of ready message when in release builds.
  [#53](https://github.com/AustinWise/smeagol/issues/53)

0.4.2 (2022-10-07)
=====

* When editing a page, add the ability to preview your how your changes will
  look before saving [#51](https://github.com/AustinWise/smeagol/issues/51)
* Update dependencies, including Rocket web framework to 0.5.0-rc.2 .

0.4.1 (2022-04-16)
=====

* Update search index on document change [#41](https://github.com/AustinWise/smeagol/issues/41)
* Basic cross-site request forgery protection [#43](https://github.com/AustinWise/smeagol/issues/43)
* Strip symbols on release build, resulting in smaller binary size for Linux [#42](https://github.com/AustinWise/smeagol/issues/42)

0.4.0 (2022-02-13)
=====

* Support reading files from a Git repo [#34](https://github.com/AustinWise/smeagol/issues/34)
* Support writing files to a Git repo [#35](https://github.com/AustinWise/smeagol/issues/35)
* Improve layout on mobile.
* Rename the settings in `smeagol.toml` to use kebob-case, rather than
  snake_case. Specifically `h1_title` was renamed to `h1-title` and `index_page`
  was renamed to `index-page`. This matches `Cargo.toml`'s use of kebob-case.
* Switch the `index-page` default value to `README`. This matches GitHub's
  behavior. Previously this value was `Home`.

0.3.0 (2022-02-10)
=====

* Add overview, which allows browsing files and folders [#8](https://github.com/AustinWise/smeagol/issues/8)
* Add basic search. Files are indexed on startup. [#23](https://github.com/AustinWise/smeagol/issues/23)
* Don't allow access to files and directories whose name starts with a dot (`.`)

0.2.1 (2021-01-19)
=====

* Add favicon

0.2.0 (2022-01-18)
=====

* Basic editing support [#16](https://github.com/AustinWise/smeagol/issues/16)
* When navigating to a non-existent markdown page, show a placeholder that allows creating that page.
* Support for serving files other than Markdown files. [#6](https://github.com/AustinWise/smeagol/issues/6)
* Support for specifying which address and port to bind the web server to.
* Add caching support for static assets. [#18](https://github.com/AustinWise/smeagol/issues/18)

0.1.1 (2022-01-12)
=====

* Add support for `h1_title` setting.
* Add breadcrumbs navigation
* Embed CSS in Smeagol, so no internet connection is required to run [#9](https://github.com/AustinWise/smeagol/issues/9)
* Add "home" button to go to the top of the site.

0.1.0 (2022-01-04)
=====

Initial proof of concept version. Basically a web server that renders markdown
pages into HTML. Supports the following features:

* Specifying which directory to serve.
* Responding to requests for `.md` files with the rendered HTML.
* Automatically redirecting to an index page if a directory is requested.
* Specifying the name of the directory page.
