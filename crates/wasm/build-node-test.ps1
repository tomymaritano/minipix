# crates/wasm/build-node-test.ps1 — Build wasm for Node.js smoke tests
# Uso: desde la raiz del workspace: .\crates\wasm\build-node-test.ps1
# Genera artefactos en target/wasm-node-test/ (nodejs target, sin wasm-opt para velocidad).

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# --- Verificar wasm-bindgen CLI (version pinada == version del crate: 0.2.123) ---
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
    Write-Host "wasm-bindgen no encontrado. Instalando 0.2.123 (puede tardar varios minutos)..."
    cargo install wasm-bindgen-cli --version 0.2.123 --locked
    if (-not $?) {
        Write-Error "Fallo al instalar wasm-bindgen-cli"
        exit 1
    }
}
$wbgVersion = (wasm-bindgen --version 2>&1).ToString().Trim()
Write-Host "wasm-bindgen: $wbgVersion"

# --- Paso 1: cargo build con el perfil wasm-release ---
Write-Host ""
Write-Host "==> Paso 1: cargo build --profile wasm-release (target wasm32-unknown-unknown)"
cargo build -p minipix-wasm --target wasm32-unknown-unknown --profile wasm-release
if ($LASTEXITCODE -ne 0) { Write-Error "cargo build fallo (exit $LASTEXITCODE)"; exit 1 }

# --- Paso 2: wasm-bindgen con target nodejs ---
Write-Host ""
Write-Host "==> Paso 2: wasm-bindgen --target nodejs"
wasm-bindgen target/wasm32-unknown-unknown/wasm-release/minipix_wasm.wasm `
    --out-dir target/wasm-node-test `
    --target nodejs `
    --out-name minipix_wasm
if ($LASTEXITCODE -ne 0) { Write-Error "wasm-bindgen fallo (exit $LASTEXITCODE)"; exit 1 }

# --- Reporte final ---
Write-Host ""
$wasm = Get-Item target/wasm-node-test/minipix_wasm_bg.wasm
"minipix_wasm_bg.wasm: $([math]::Round($wasm.Length / 1MB, 2)) MB raw ($($wasm.Length) bytes)"
Write-Host "Artefactos en: target/wasm-node-test/"
Get-ChildItem target/wasm-node-test | Select-Object Name, Length | Format-Table -AutoSize
Write-Host "Build completado exitosamente."
