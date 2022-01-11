WIP
===

* Add support for `h1_title` setting.
* Add breadcrumbs navigation

0.1.0 (2022-01-04)
=====

Initial proof of concept version. Basically a web server that renders markdown
pages into HTML. Supports the following features:

* Specifying which directory to serve.
* Responding to requests for `.md` files with the rendered HTML.
* Automatically redirecting to an index page if a directory is requested.
* Specifying the name of the directory page.
