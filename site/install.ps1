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
    [string]$Version,
    [Parameter(HelpMessage = 'Which directory to download file to.')]
    [string]$InstallDir
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

function Detect-Target {
    try {
        # Despite the "mscorlib", this also works in Powershell Core
        $arch = [System.Runtime.InteropServices.RuntimeInformation, mscorlib]::OSArchitecture
    }
    catch {
        # TODO: consider using GetNativeSystemInfo directly, to support older versions of the .NET .
        # The only supported Windows 10 releases that did not come with this feature are LTSC releases,
        # so maybe it is not a big deal.
        throw "This installer requires .NET Framework 4.7.1 or later. Please install from here: https://dotnet.microsoft.com/download/dotnet-framework"
    }

    switch ($arch) {
        ([System.Runtime.InteropServices.Architecture, mscorlib]::X86) { $target_arch = "i686" }
        ([System.Runtime.InteropServices.Architecture, mscorlib]::X64) { $target_arch = "x86_64" }
        ([System.Runtime.InteropServices.Architecture, mscorlib]::Arm) { throw "As of Rust 1.58, 32-bit ARM on Windows is not supported. thumbv7a-pc-windows-msvc only has tier 3 support: https://doc.rust-lang.org/nightly/rustc/platform-support.html" }
        ([System.Runtime.InteropServices.Architecture, mscorlib]::Arm64) { $target_arch = "aarch64" }
        # TODO: add a more useful help message, like saying to file a bug or update something
        Default { throw "Unknown CPU architecture: $arch" }
    }

    return "$target_arch-pc-windows-msvc"
}

$file_extractor_source = @"
using System;
using System.IO;
using System.IO.Compression;
using System.Linq;

namespace TEMP_NAMESPACE_REPLACE_ME
{
    public class FileExtractor
    {
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
                        Console.WriteLine(tempFileName);
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
            catch (Exception ex)
            {
                Console.WriteLine(ex);
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
Add-Type -TypeDefinition $file_extractor_source -ReferencedAssemblies @("System.IO.Compression", "System.IO.Compression.FileSystem")
$file_extractor = new-object -TypeName "$file_extractor_namespace.FileExtractor"

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



# TODO: write to the correct location and make sure it is in the PATH
$output_file = "C:\temp\$Crate.exe"
$file_extractor.ExtractExe($temp_file_path, "$Crate.exe", $output_file)
Write-Host "Extracted to: $output_file"
