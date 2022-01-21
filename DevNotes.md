
Notes that might be helpful when hacking on this program.

# Documentation for libraries used

* [Primer for the design system and CSS](https://primer.style)
* [Rocket docs](https://rocket.rs)
* [Askama template syntax](https://djc.github.io/askama/template_syntax.html)

When hacking on GitHub actions workflows, these might be helpful:

* [GitHub Actions documentation](https://docs.github.com/en/actions)
* [GitHub API for releases](https://docs.github.com/en/rest/reference/releases)

# Releasing a new version

Perform the following steps:

1. Update `CHANGELOG.md` with any missing issues, the version number, and the release date.
1. Check that `Cargo.toml` has the right version.
1. `cargo build` to ensure `Cargo.lock` is also updated
1. `git commit`
1. `git push origin master`
1. `git tag -a x.x.x` , where `x.x.x` is the version. The body should be the changelog for this
    release. It should look roughly like this (like a Git commit).

    ```
    Release x.x.x

    Changes:
    * thing 1
    * thing 2
    * thing 3
    ```
1. `git push origin x.x.x`
1. `cargo publish`
1. Edit `Cargo.toml` and `Changelog.md` with a new version number
1. `cargo build` to ensure `Cargo.lock` is also updated
1. `git commit`
1. `git push origin master`

# Profiling binary size using SizeBench

Microsoft's [SizeBench](https://www.microsoft.com/store/productId/9NDF4N1WG7D6)
tool can look at the binary. You have to make the following changes to the linking
on Windows to make the generated code compatible.

Add these to the `.cargo/config` file:

```
rustflags = ["-C", "link-arg=/LTCG:OFF", "-C", "link-arg=/INCREMENTAL:NO"]
```

Add this to `Cargo.toml`:

```toml
[profile.release]
codegen-units = 1
```
