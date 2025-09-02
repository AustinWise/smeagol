# Smeagol - locally hosted wiki

The goal of this project is to create a wiki software with these properties:

* Compatible with GitHub. This means:
  * Git is the backing store
  * Markdown is used to format pages
* Easy to run on your local computer.
* Really fast and easy to install.

Non goals include:

* Support for multiple users.

Phrased in another way, the goal of this project is to create something that behaves
like [Gollum](https://github.com/gollum/gollum) be installs easily and quickly. This is contrasted
with the Gollum install experience of having to deal with the slowness and complication of setting
up a Ruby environment and running `gem install`.

## Getting started

See the [smeagol.dev website](https://smeagol.dev/) for more install options.

If you use the Rust programming language, you can also install this tool using Cargo:

```bash
cargo install smeagol-wiki
```

Nix users can also install it through nixpkgs:

```bash
nix-env --install --attr nixpkgs.smeagol
```

Download the [latest release from GitHub](https://github.com/AustinWise/smeagol/releases/latest).
Extract the `smeagol-wiki` executable from the compressed archive.
`smeagol-wiki` is a command line application. It needs a directory
containing the Markdown files you want to serve. You can pass
a command line argument to it to specify the directory:

```bash
smeagol-wiki ~/wiki
```

When run without arguments, the current directory is used.

Once started, it listens on http://127.0.0.1:8000 by default.

## Configuration

There are a few command line options:

* `--host` - takes an argument that specifies which IP address to bind to. By
  default this is `127.0.0.1`, which means only users on your local computer can
  access the wiki. Set to `0.0.0.0` to let other computers on your network
  access it.
* `--port` - takes an argument that specifies which port to listen on. `8000` by
  default.
* `--fs` - instructs Smeagol to load and save using the file system. By default
  Smeagol uses Git to load files committed to a Git repository and saves them by
  committing them to the current branch.

Additionally, the following settings can be put in a `smeagol.toml` file in the
root directory of the wiki:

* `index-page` - By default `README`. When you browse to a directory, Smeagol
  will display a file whose name (not including the extension) is `index-page`.
  For example, when you navigate to `/page/for/bar/`, Smeagol will display the
  file at `foo/bar/README.md`.
* `h1-title` - By default `false`. When false, Smeagol will use the file name
  as the title of the page. This title will be displayed at the top of the page
  and in the title bar. When this setting is true and a document starts with an
  `h1` (written as a line that starts with `#` in Markdown), the text of this
  `h1` will be used as the title of the page. It will not be rendered as a
  normal part of the document.

## Differences from Gollum

* `index-page` on Gollum defaults to `Home`. Smeagol defaults to `README` to be
  compatible with online code hosting systems such as GitHub and Azure Devops.
* The default port is `8000` rather than `4567`.
* Support for transclusion. If a line contains `{{file-name.md}}`, the contents of `file-name.md`
  will replace that line.

## Security

Smeagol is intended to be run on your local computer to read your own private data. It is not
designed to be exposed to the public internet: there is no authentication.

That said, there is one class of security problem I would be interested in hearing about: opening
a maliciously designed Wiki with Smeagol either causing code execution or writing to files outside
the wiki directory. Please file an issue if you encounter such a problem.

## Why Rust, please tell me more about why you love Rust

Rust makes it easy to ship cross-compiled executables that run without much fuss.
As for why not some other language also shares this capability (Go or C#),
I just want to get more experience working with Rust.

## License

Licensed under the [MIT License](LICENSE).

Note that some elements, specifically aspects of the visual design,
have been copied from [Gollum](https://github.com/gollum/gollum).
