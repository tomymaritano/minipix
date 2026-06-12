# crates/wasm/build.ps1 — build del binding para el playground
# Uso: desde la raíz del workspace: .\crates\wasm\build.ps1
# Nota: wasm-pack 0.15 no soporta perfiles custom (--profile wasm-release) de forma
# directa via CLI; usa --release y los ajustes de [profile.wasm-release] se aplican
# via [package.metadata.wasm-pack.profile.release] para wasm-opt.

# Refrescar PATH para asegurar que wasm-pack y clang están disponibles
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Verificar que wasm-pack está disponible; instalar si no
if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    Write-Host "wasm-pack no encontrado. Instalando (puede tardar varios minutos)..."
    cargo install wasm-pack --locked
    if (-not $?) {
        Write-Error "Fallo al instalar wasm-pack"
        exit 1
    }
}

# Build del binding wasm
# --release: wasm-pack 0.15 no acepta perfiles custom por CLI.
# El perfil wasm-release del workspace (opt-level=z, lto=fat, panic=abort, strip=symbols)
# aplica cuando se invoca directamente con: cargo build --target wasm32-unknown-unknown --profile wasm-release
# Para wasm-pack usamos --release y wasm-opt se aplica via package.metadata.
wasm-pack build crates/wasm `
    --target web `
    --out-dir ../../playground/src/lib/wasm `
    --release

if ($?) {
    Write-Host ""
    Write-Host "Build exitoso. Artefactos en: playground/src/lib/wasm/"
    $wasmFile = "playground/src/lib/wasm/minipix_wasm_bg.wasm"
    if (Test-Path $wasmFile) {
        $size = (Get-Item $wasmFile).Length
        Write-Host "Tamanio .wasm: $size bytes ($([math]::Round($size/1MB, 2)) MB)"
    }
} else {
    Write-Error "Build fallido"
    exit 1
}
