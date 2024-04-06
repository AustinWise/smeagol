
# Notes about detecting Windows architecture in PowerShell

Update for year 2024: at this point, the method described in the footnote at the end of this article
is probably good enough for most user-mode applications. It is only missing on older versions of
Windows and only gives the wrong answer on earlier versions of ARM64 Windows 11. Given the current
low levels of adoption of Windows on Arm64, and the minor consequences of getting the answer wrong
(your program will run slightly slower in emulation), it's good enough.

This explains why the logic for detecting the OS native architecture is
so convoluted in the [PowerShell install script](site/install.ps1).

## What's wrong with `RuntimeInformation`

The
[RuntimeInformation](https://learn.microsoft.com/dotnet/api/system.runtime.interopservices.runtimeinformation.osarchitecture)
seems like the obvious way to detect architecture:

```powershell
[System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
```

There are a number of problems with this though:

* This API was added in .NET 4.7.1, which ships with Windows 10 v1709. This
  means old, unpatched versions of Windows 10 and 8 cannot use this API in
  PowerShell. And on Windows 7 were PowerShell runs on .NET 3.5, it cannot be used
  at all.
* Until very recent versions of Windows and PowerShell Core, it gives incorrect
  answers when running on ARM64 Windows in emulation. Notably, prior to Windows
  11 22H2, the only version of .NET Framework for ARM64 ran as x64 in emulation,
  so classic PowerShell will always give the wrong answer. See
  [this issue](https://github.com/dotnet/runtime/issues/26612) and
  [this PR](https://github.com/dotnet/runtime/pull/60910) which fixed the issue
  in .NET 7.
* On classic PowerShell you need to use reflection to read this property.
  See footnote.

## Windows APIs for determining version

Fortunately we can use P/Invoke in PowerShell to call Win32 APIs to reliably
discover the architecture of the system:

* [`GetMachineTypeAttributes`](https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-getmachinetypeattributes)
  Added in Windows 11. The most flexible API. It allows you to pass an architecture
  in and figure out if it is support either natively or in emulation.
* [`IsWow64Process2`](https://learn.microsoft.com/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2)
  Added in Windows 10 v1511. Tells the truth (currently) about the native OS
  architecture.
* [`GetNativeSystemInfo`](https://learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo)
  Added in Windows XP. Lies about the native system when running under x886
  emulation on ARM64 and in ARM64EC processes.

## Footnote: why we need to use reflection to access `RuntimeInformation`

You would think we could just uses the `RuntimeInformation` class thusly:

```powershell
[System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
```

This works in PowerShell Core, but not the .NET Framework version of PowerShell
that comes with Windows. We can see the problem here:

```powershell
[System.Runtime.InteropServices.RuntimeInformation].Assembly
# prints Microsoft.PowerShell.PSReadLine.dll . It should be in mscorlib.
```

Therefore we need to use reflection to make sure we load the type from the
correct assembly:

```powershell
$a = [System.Reflection.Assembly]::LoadWithPartialName("System.Runtime.InteropServices.RuntimeInformation")
$t = $a.GetType("System.Runtime.InteropServices.RuntimeInformation")
$p = $t.GetProperty("OSArchitecture")
$p.GetValue($null)
```
