
Notes that might be helpful when hacking on this program.

# Documentation for libraries used

* [Primer for the design system and CSS](https://primer.style)
* [Rocket docs](https://rocket.rs)
* [Askama template syntax](https://djc.github.io/askama/template_syntax.html)

# Releasing a new version

Perform the following steps:

1. Update `CHANGELOG.md` with any missing issues, the version number, and the release date.
1. `git push origin master`
1. `git tag -a x.x.x` , where `x.x.x` is the version. The body should be the changelog for this release.
1. `git push origin x.x.x`
1. `cargo publish`
1. Edit `Cargo.toml` and `Changelog.md` with a new version number
1. `git push origin master`

