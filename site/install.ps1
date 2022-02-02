param (
    [Parameter(Mandatory = $true,
    HelpMessage = 'Enter the name of the GitHub repo. For example: AustinWise/smeagol')]
    [string]$GitHub,
    [Parameter(HelpMessage = 'The target triple, for example x86_64-pc-windows-msvc. By default this is detected automatically.')]
    [string]$Target,
    [Parameter(HelpMessage = 'The name of the crate to download. By default it is the same as the repository name.')]
    [string]$Crate,
    [Parameter(HelpMessage = 'Whether to show a progress bar. Disabled by default because it greatly increase the amount of time a download takes.')]
    [switch]$ShowProgressBar,
    [Parameter(HelpMessage = 'Which version to download. By default the latest version is downloaded')]
    [string]$Version
)

$ErrorActionPreference = "Stop"

if (!$ShowProgressBar) {
    $ProgressPreference = "SilentlyContinue"
}

#TODO, maybe set [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12;
# This will allow running on older .NET Framework versions

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    # TODO: show the precise shell incantation to use
    throw "This script is only supported on Windows. On unix-like systems, use the shell script.";
}

$bin_folder_name = ".kame-app"
$bin_dir = [System.IO.Path]::Combine($env:USERPROFILE, $bin_folder_name)
if (!(Test-Path $bin_dir)) {
    mkdir $bin_dir -Force
}

$file_extractor_source = @"
using Microsoft.Win32.SafeHandles;
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Runtime.InteropServices;

namespace TEMP_NAMESPACE_REPLACE_ME
{
    public class Helper
    {
        [DllImport("kernel32", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool IsWow64Process2(SafeProcessHandle handle, out ushort pProcessMachine, out ushort pNativeMachine);

        [StructLayout(LayoutKind.Sequential)]
        struct SYSTEM_INFO
        {
            public ushort wProcessorArchitecture;
            ushort wReserved;
            int dwPageSize;
            IntPtr lpMinimumApplicationAddress;
            IntPtr lpMaximumApplicationAddress;
            IntPtr dwActiveProcessorMask;
            int dwNumberOfProcessors;
            int dwProcessorType;
            int dwAllocationGranularity;
            short wProcessorLevel;
            short wProcessorRevision;
        }

        const int PROCESSOR_ARCHITECTURE_INTEL = 0;
        const int PROCESSOR_ARCHITECTURE_ARM = 5;
        const int PROCESSOR_ARCHITECTURE_AMD64 = 9;
        const int PROCESSOR_ARCHITECTURE_ARM64 = 12;

        [DllImport("kernel32")]
        static extern void GetNativeSystemInfo(out SYSTEM_INFO lpSystemInfo);

        public int GetArchitecture()
        {
            try
            {
                ushort process, native;
                var cur_proc = Process.GetCurrentProcess();
                if (!IsWow64Process2(cur_proc.SafeHandle, out process, out native))
                    throw new Win32Exception();
                switch (native)
                {
                    case 0x014c:
                        return PROCESSOR_ARCHITECTURE_INTEL;
                    case 0x8664:
                        return PROCESSOR_ARCHITECTURE_AMD64;
                    case 0xAA64:
                        return PROCESSOR_ARCHITECTURE_ARM64;
                    //TODO:figure out which is correct
                    case 0x01c0: //ARM Little-Endian
                    case 0x01c2: //ARM Thumb/Thumb-2 Little-Endian
                        return PROCESSOR_ARCHITECTURE_ARM;
                    default:
                        throw new Exception("Unknown architecture: " + native.ToString("X"));
                }
            }
            catch (EntryPointNotFoundException)
            {
                // On OSs earlier than Windows 10, version 1511, fall through to the old way.
            }

            // NOTE: When called from ARM64EC mode, this function gives the wrong answer!
            // Specifically it return x86_64. And as of Windows 11, PowerShell.exe is a
            // ARM64EC process.
            SYSTEM_INFO sysInfo;
            GetNativeSystemInfo(out sysInfo);
            return sysInfo.wProcessorArchitecture;
        }

        public void ExtractExe(string zipFilePath, string exeFileName, string pathToTarget)
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(pathToTarget));
                using (var zip = ZipFile.OpenRead(zipFilePath))
                {
                    var matchingEntries = zip.Entries.Where(e => e.Name == exeFileName).ToList();
                    if (matchingEntries.Count == 0)
                    {
                        throw new Exception("The downloaded zip file did not contain the expect exe. Looked for: " + exeFileName);
                    }
                    else if (matchingEntries.Count == 1)
                    {
                        string tempFileName = pathToTarget + ".new";
                        using (var tempFs = new FileStream(tempFileName, FileMode.Create, FileAccess.Write, FileShare.None))
                        using (var exeFs = matchingEntries[0].Open())
                        {
                            exeFs.CopyTo(tempFs);
                            tempFs.Flush(true);
                        }
                        if (File.Exists(pathToTarget))
                        {
                            File.Replace(tempFileName, pathToTarget, pathToTarget + ".old");
                        }
                        else
                        {
                            File.Move(tempFileName, pathToTarget);
                        }
                    }
                    else
                    {
                        throw new Exception("The downloaded zip contained multiple copies of the exe. I don't know which one to use. Found paths: " + string.Join(", ", matchingEntries.Select(e => e.FullName).ToArray()));
                    }
                }
            }
            catch
            {
                throw;
            }
            finally
            {
                File.Delete(zipFilePath);
            }
        }
    }
}
"@

$file_extractor_namespace = "NS_" + [System.Guid]::NewGuid().ToString("N")
$file_extractor_source = $file_extractor_source.Replace("TEMP_NAMESPACE_REPLACE_ME", $file_extractor_namespace)
Add-Type -TypeDefinition $file_extractor_source -ReferencedAssemblies @("System.IO.Compression", "System.IO.Compression.FileSystem", "System.Diagnostics.Process", "System.Linq", "System.ComponentModel.Primitives", "Microsoft.Win32.Primitives", "System.IO.Compression.ZipFile", "System.Collections")
$helper = new-object -TypeName "$file_extractor_namespace.helper"

function Detect-Target {
    switch ($helper.GetArchitecture()) {
        0 { $target_arch = "i686" }
        5 { throw "As of Rust 1.58, 32-bit ARM on Windows is not supported. thumbv7a-pc-windows-msvc only has tier 3 support: https://doc.rust-lang.org/nightly/rustc/platform-support.html" }
        9 { $target_arch = "x86_64" }
        12 { $target_arch = "aarch64" }
        # TODO: add a more useful help message, like saying to file a bug or update something
        Default { throw "Unknown CPU architecture: $arch" }
    }

    return "$target_arch-pc-windows-msvc"
}

function Detect-LatestVersion() {
    $url = "https://api.github.com/repos/$GitHub/releases/latest"
    $headers = @{"Accept" = "application/vnd.github.v3+json"; "User-Agent" = "Install AustinWise/smeagol"}
    if ($null -ne $env:GITHUB_TOKEN) {
        write-host "Using GitHub token"
        $headers["Authorization"] = "token $env:GITHUB_TOKEN"
    }
    
    try {
        # TODO: switch to using invoke-restmethod
        $data = Invoke-WebRequest -Uri $url -Headers $headers -UseBasicParsing
    }
    catch {
        throw "Failed to find the latest release from $url : $_"
    }
    $response = $data.Content
    $response = ConvertFrom-Json $response
    return $response.Name
}

$split = $GitHub.Split("/")
if ($split.Length -ne 2) {
    throw "Invalid GitHub name, expected something like org_name/repo_name, found: $GitHub"
}

# $org_name = $split[0]
$repo_name = $split[1]

if ([System.String]::IsNullOrEmpty($Crate)) {
    $Crate = $repo_name
}

if ([System.String]::IsNullOrEmpty($Target)) {
    $Target = Detect-Target
}

if ([System.String]::IsNullOrEmpty($Version)) {
    $Version = Detect-LatestVersion
}

$download_url = "https://github.com/$GitHub/releases/download/$Version/$Crate-$Version-$Target.zip"
$temp_guid = [System.Guid]::NewGuid()
$temp_file_path = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), "$temp_guid.zip")

Write-Host "Downloading version $Version"
Write-Debug "Downloading from: $download_url"
Write-Debug "Downloaded to: $temp_file_path"

try {
    Invoke-WebRequest -Uri $download_url -UseBasicParsing -Out $temp_file_path
} catch {
    throw "Failed to download from $download_url to  $temp_file_path, error: $_"
}

$output_file = [System.IO.Path]::Combine($bin_dir, "$Crate.exe")
$helper.ExtractExe($temp_file_path, "$Crate.exe", $output_file)
Write-Host "Extracted to: $output_file"

$path_entry = "$env:USERPROFILE\$bin_folder_name"
$existing_path = [System.Environment]::GetEnvironmentVariable("PATH", [System.EnvironmentVariableTarget]::User);
$ndx = $existing_path.IndexOf($path_entry, [System.StringComparison]::OrdinalIgnoreCase)
if ($ndx -lt 0) {
    Write-Host "Adding $path_entry to PATH"
    $split = $existing_path.Split(";")
    $split += $path_entry
    $joined = [System.String]::Join(";", $split)
    [System.Environment]::SetEnvironmentVariable("PATH", $joined, [System.EnvironmentVariableTarget]::User);
    Write-Host "Update PATH. Please restart your terminals and command prompts."
}

Write-Host "Run the program by typing '$Crate' in a terminal."
