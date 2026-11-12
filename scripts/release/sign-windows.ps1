<#
  Authenticode signing for one or more files (M9 T9, ADR-0082).

    pwsh scripts/release/sign-windows.ps1 <file> [<file> ...]

  Tauri calls this through bundle.windows.signCommand for the executables and the MSI; the
  release workflow calls it for mosaic.exe and mosaic-native-host.exe.

  WINDOWS_SIGN_MODE selects the method:
    none (or unset)    no signing: a warning, exit 0. This is the default (OD-M9-2).
    trusted-signing    Azure Trusted Signing through `trusted-signing-cli`; needs
                       AZURE_TENANT_ID, AZURE_CLIENT_ID (and the workload identity or secret
                       the Azure SDK reads), AZURE_TRUSTED_SIGNING_ENDPOINT,
                       AZURE_TRUSTED_SIGNING_ACCOUNT and AZURE_CERT_PROFILE.
    pfx                a certificate file: WINDOWS_PFX_BASE64 and WINDOWS_PFX_PASSWORD, signed
                       with signtool (SHA-256, RFC 3161 timestamp).
  Any other value, or a missing variable for the chosen mode, is an error: a half-configured
  signing setup never produces an unsigned release by accident.

  When MOSAIC_UNSIGNED_HASHES names a file, the SHA-256 of each file is appended to it before
  signing, in `sha256sum` format (the pre-signing hashes of SHA256SUMS.unsigned, R120).
  No secret is printed and nothing is traced.
#>
[CmdletBinding()]
param([Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Files)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$mode = if ($env:WINDOWS_SIGN_MODE) { $env:WINDOWS_SIGN_MODE } else { 'none' }
$timestampUrl = 'http://timestamp.digicert.com'

function Require-Env([string[]]$Names) {
  foreach ($n in $Names) {
    if (-not [Environment]::GetEnvironmentVariable($n)) { throw "WINDOWS_SIGN_MODE=$mode needs $n" }
  }
}

function Find-SignTool {
  $found = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($found) { return $found.Source }
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  $candidate = Get-ChildItem -Path $kits -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1
  if (-not $candidate) { throw 'signtool.exe was not found' }
  return $candidate.FullName
}

foreach ($f in $Files) {
  if (-not (Test-Path -LiteralPath $f -PathType Leaf)) { throw "not a file: $f" }
  if ($env:MOSAIC_UNSIGNED_HASHES) {
    $h = (Get-FileHash -Algorithm SHA256 -LiteralPath $f).Hash.ToLowerInvariant()
    Add-Content -Encoding ascii -Path $env:MOSAIC_UNSIGNED_HASHES -Value ("{0}  {1}" -f $h, (Split-Path -Leaf $f))
  }
}

switch ($mode) {
  'none' {
    Write-Host "::warning::unsigned: WINDOWS_SIGN_MODE is none, $($Files.Count) file(s) left without Authenticode"
    exit 0
  }
  'trusted-signing' {
    Require-Env @('AZURE_TENANT_ID', 'AZURE_CLIENT_ID', 'AZURE_TRUSTED_SIGNING_ENDPOINT', 'AZURE_TRUSTED_SIGNING_ACCOUNT', 'AZURE_CERT_PROFILE')
    foreach ($f in $Files) {
      & trusted-signing-cli -e $env:AZURE_TRUSTED_SIGNING_ENDPOINT -a $env:AZURE_TRUSTED_SIGNING_ACCOUNT `
        -c $env:AZURE_CERT_PROFILE -d 'Mosaic' $f
      if ($LASTEXITCODE -ne 0) { throw "trusted-signing-cli failed for $f" }
    }
  }
  'pfx' {
    Require-Env @('WINDOWS_PFX_BASE64', 'WINDOWS_PFX_PASSWORD')
    $signtool = Find-SignTool
    $pfx = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName() + '.pfx')
    try {
      [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_PFX_BASE64))
      foreach ($f in $Files) {
        # signtool takes the password as an argument; it is only visible to this runner process.
        & $signtool sign /fd SHA256 /f $pfx /p $env:WINDOWS_PFX_PASSWORD /tr $timestampUrl /td SHA256 /d 'Mosaic' $f
        if ($LASTEXITCODE -ne 0) { throw "signtool sign failed for $f" }
      }
    } finally {
      if (Test-Path -LiteralPath $pfx) { Remove-Item -LiteralPath $pfx -Force }
    }
  }
  default { throw "unknown WINDOWS_SIGN_MODE '$mode' (none, trusted-signing or pfx)" }
}

foreach ($f in $Files) {
  $sig = Get-AuthenticodeSignature -LiteralPath $f
  if ($sig.Status -ne 'Valid') { throw "Authenticode check failed for ${f}: $($sig.Status)" }
  if (-not $sig.TimeStamperCertificate) { throw "no RFC 3161 timestamp on $f" }
  Write-Host "signed and verified: $f"
}
