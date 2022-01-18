WIP
===

* Support for serving files other than Markdown files. [#6](https://github.com/AustinWise/smeagol/issues/6)
* Support for specifying which address and port to bind the web server to.

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