# crates/wasm/build.ps1 — pipeline manual con el perfil wasm-release real
# Uso: desde la raiz del workspace: .\crates\wasm\build.ps1
#
# NOTA: wasm-pack --release usa el perfil "release" plano de cargo y NO mapea perfiles
# custom (e.g. wasm-release) a su metadata de wasm-opt; el flag --profile existe en CLI
# pero no transfiere la configuracion al paso de wasm-opt (-O default). Verificado en
# review M2-T3. Pipeline manual: cargo (perfil wasm-release) + wasm-bindgen + wasm-opt.

$ErrorActionPreference = "Stop"
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

# --- Resolver wasm-opt: PATH primero, luego cache de wasm-pack en AppData ---
$wasmOpt = $null
$wasmOptCmd = Get-Command wasm-opt -ErrorAction SilentlyContinue
if ($wasmOptCmd) {
    $wasmOpt = $wasmOptCmd.Source
    Write-Host "wasm-opt (PATH): $wasmOpt"
} else {
    # wasm-pack 0.15 descarga wasm-opt en %LOCALAPPDATA%\.wasm-pack\wasm-opt-<hash>\bin\
    $cachePath = "$env:LOCALAPPDATA\.wasm-pack"
    if (Test-Path $cachePath) {
        $found = Get-ChildItem $cachePath -Recurse -Filter "wasm-opt.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) {
            $wasmOpt = $found.FullName
            Write-Host "wasm-opt (wasm-pack cache): $wasmOpt"
        }
    }
}
if (-not $wasmOpt) {
    Write-Error "wasm-opt no encontrado. Instala binaryen (https://github.com/WebAssembly/binaryen/releases) o ejecuta 'wasm-pack build' una vez para descargarlo."
    exit 1
}

# --- Paso 1: cargo con el perfil de tamano ---
Write-Host ""
Write-Host "==> Paso 1: cargo build --profile wasm-release"
cargo build -p minipix-wasm --target wasm32-unknown-unknown --profile wasm-release
if (-not $?) { Write-Error "cargo build fallo"; exit 1 }

# --- Paso 2: wasm-bindgen ---
Write-Host ""
Write-Host "==> Paso 2: wasm-bindgen"
wasm-bindgen target/wasm32-unknown-unknown/wasm-release/minipix_wasm.wasm `
    --out-dir playground/src/lib/wasm `
    --target web `
    --out-name minipix_wasm
if (-not $?) { Write-Error "wasm-bindgen fallo"; exit 1 }

# --- Paso 3: wasm-opt (features que Rust 1.87+ emite por default) ---
Write-Host ""
Write-Host "==> Paso 3: wasm-opt"
& $wasmOpt playground/src/lib/wasm/minipix_wasm_bg.wasm `
    -o playground/src/lib/wasm/minipix_wasm_bg.wasm `
    -Oz --enable-bulk-memory --enable-nontrapping-float-to-int
if (-not $?) { Write-Error "wasm-opt fallo"; exit 1 }

# --- Reporte final ---
Write-Host ""
$wasm = Get-Item playground/src/lib/wasm/minipix_wasm_bg.wasm
"minipix_wasm_bg.wasm: $([math]::Round($wasm.Length / 1MB, 2)) MB raw ($($wasm.Length) bytes)"
Write-Host "Artefactos en: playground/src/lib/wasm/"
Get-ChildItem playground/src/lib/wasm | Select-Object Name, Length | Format-Table -AutoSize
