# Release $VERSION

## Installation

Download the appropriate binary for your platform from the assets below.

### Binary Downloads

See assets below for pre-built binaries for your platform.

### Verify checksums

Each binary includes a `.sha256` file for verification:

```bash
# Linux/macOS
sha256sum -c mdlint-*.sha256

# Windows (PowerShell)
$expected = (Get-Content mdlint-*.sha256).Split()[0]
$actual = (Get-FileHash mdlint.exe).Hash.ToLower()
if ($expected -eq $actual) { "OK" } else { "FAILED" }
```

### Verify with packslip

Each release is also signed with [packslip](https://packslip.dev) as `packslip.sigstore.json`.
Install packslip via mise (`mise install`) and verify the downloaded binary:

```bash
curl -fsSLO https://github.com/nnutter/mdlint/releases/download/v$VERSION/packslip.sigstore.json
curl -fsSLO https://github.com/nnutter/mdlint/releases/download/v$VERSION/mdlint-linux-x86_64.tar.gz
packslip verify packslip.sigstore.json --artifact mdlint-linux-x86_64.tar.gz
```
