The marketing site.

Stores the `install.sh` script to make it easy to install.

## Install Script Notes

Invoke from a power shell instance:

```
iex "& { $(irm https://smeagol.dev/install.ps1 ) } -GitHub AustinWise/smeagol -Crate smeagol-wiki"
```

Invoke from CMD.EXE:

```
powershell -ExecutionPolicy ByPass -NoProfile -Command "iex \" ^& { $(irm https://smeagol.dev/install.ps1 ) } -GitHub AustinWise/smeagol -Crate smeagol-wiki\" "
```

## TODO

* Some sort of real build system to pull the CSS in? Perhaps `esbuild`?
