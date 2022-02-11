0.3.0 (2022-02-10)
===

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
