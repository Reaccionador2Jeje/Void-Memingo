$ErrorActionPreference = "Stop"

$PackageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$Payload = Join-Path $PackageRoot "payload"
$GrubDirectory = Join-Path $Payload "boot\grub"

if (-not (Test-Path -LiteralPath $GrubDirectory -PathType Container)) {
    throw "Void Boot payload was not found. Extract the complete ZIP before running install.ps1."
}

$Target = Read-Host "Destination directory (existing folder or mounted USB drive)"
if ([string]::IsNullOrWhiteSpace($Target)) {
    throw "No destination selected."
}

$Target = [System.IO.Path]::GetFullPath($Target)
$ProtectedPaths = @($env:SystemRoot, $env:ProgramFiles, ${env:ProgramFiles(x86)}) | Where-Object { $_ }
foreach ($ProtectedPath in $ProtectedPaths) {
    if ($Target.TrimEnd("\") -ieq $ProtectedPath.TrimEnd("\")) {
        throw "Refusing to deploy into a protected Windows system directory: $Target"
    }
}
if (-not (Test-Path -LiteralPath $Target -PathType Container)) {
    New-Item -ItemType Directory -Path $Target -Force | Out-Null
}

Write-Host "This copies menu and profile files into: $Target"
Write-Host "It does not install GRUB, partition or format a drive, or make the destination bootable."
$Confirm = Read-Host "Continue? [y/N]"
if ($Confirm -notin @("y", "Y", "yes", "YES")) {
    Write-Host "Cancelled."
    exit 0
}

Get-ChildItem -LiteralPath $Payload -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $Target -Recurse -Force
}
Set-Content -LiteralPath (Join-Path $Target "VOID-BOOT-INSTALL.txt") -Value "Void Boot menu/profile files deployed."
Write-Host "Void Boot files copied successfully."
