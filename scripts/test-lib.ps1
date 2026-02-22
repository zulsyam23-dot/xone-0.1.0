param(
    [int]$MaxRetries = 4
)

$ErrorActionPreference = "Continue"

function Invoke-XoneLibTests {
    $env:CARGO_INCREMENTAL = "0"
    $env:RUST_TEST_THREADS = "1"
    $env:CARGO_TARGET_DIR = "target/test-lib"

    $output = & cargo test --lib --jobs 1 -- --test-threads=1 2>&1
    $code = $LASTEXITCODE
    return @{
        Code = $code
        Output = ($output -join "`n")
    }
}

for ($attempt = 1; $attempt -le $MaxRetries; $attempt++) {
    Write-Host "test-lib attempt $attempt/$MaxRetries"
    $result = Invoke-XoneLibTests
    if ($result.Code -eq 0) {
        Write-Host $result.Output
        exit 0
    }

    $isLinkLock = $result.Output -match "LNK1104" -and $result.Output -match "xone-.*\.exe"
    if (-not $isLinkLock -or $attempt -eq $MaxRetries) {
        Write-Host $result.Output
        exit $result.Code
    }

    Write-Host "detected transient linker lock (LNK1104), retrying..."
    Start-Sleep -Milliseconds (400 * $attempt)
}

exit 1
