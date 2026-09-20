# Playground UI system — Design Doc

- **Fecha**: 2026-09-20
- **Estado**: Aprobado en brainstorming; pendiente de plan de implementación
- **Alcance**: reskin + shell del playground Svelte. No se toca el core, los workers, ni el pipeline WASM.
- **Approach**: A — mismo flujo (drop → compare → download), un sistema visual y de componentes, deploy estático.

## 1. Resumen

El playground es una SPA Vite + Svelte 5 que comprime imágenes 100% en el navegador. La UI actual mezcla dos paletas, radios y tamaños mágicos, SVG inline y primitivas shadcn sin usar (`Card`, `Tooltip`). El mint actúa como atmósfera (glow, grain, rombo) y el chrome flota encima del visor.

Esta pasada unifica tokens, arma un shell de tres bandas y obliga a que todo control pase por `$lib/components/ui`. El motor de compresión, los goldens y el host de Cloudflare Pages no cambian.

## 2. Motivación

- Consistencia: un control, un componente, una escala.
- Legibilidad: el lienzo oscuro sirve a la imagen, no al branding.
- Firma: el split before/after es lo memorable, no el glow.
- Host: el artefacto sigue siendo estático. Next.js quedó descartado.

## 3. Decisiones cerradas

| Decisión | Valor |
| --- | --- |
| Stack | Vite 8 + Svelte 5 + Tailwind v4 + bits-ui. Sin Next.js, sin SvelteKit. |
| Flujo | Empty → drop/paste/⌘O → compare → download. Atajos actuales. |
| Tokens | Una fuente: `@theme` en `app.css`. Mint = señal, no atmósfera. |
| Tipo | IBM Plex Mono only. Escala 10 / 12 / 13 / 18 / 28. |
| Radio | 4 / 8 / 12. Espacio: grilla de 4px. |
| Shell | Tres bandas: top 48px, stage `flex-1`, toolbar 56px. Padding viewport 8px. |
| Empty state | El stage entero es el dropzone, no una card flotante. |
| Stage | Sin `max-h: 640` / `max-w: 920`. Imagen `object-contain` sobre checker. |
| Zoom | Nuevo `Segmented` (`1× 2× 4×`). |
| Iconos | Lucide (`@lucide/svelte`). Cero SVG de path sueltos en pantallas. |
| Tooltip | Solo el warning AVIF. Los labels del toolbar ya son visibles. |
| Card | Stage y dropzone. |
| Core / WASM / workers / protocolo | Fuera. Resize sigue en el worker vía canvas. |
| Deploy | Cloudflare Pages workflow intacto. Vercel posible como estático; no se agrega proyecto ni `vercel.json` en esta pasada. |
| Light mode, filmstrip, batch zip, `effort` en UI, logo nuevo | Fuera. |

## 4. Sistema visual

### 4.1 Paleta

| Token semántico (`@theme`) | Hex / valor | Uso |
| --- | --- | --- |
| `--color-background` | `#0E1013` | canvas, checker base |
| `--color-card` | `#16191E` | top chrome, toolbar, stage chrome |
| `--color-secondary` | `#1C2026` | controles raised, chips activos |
| `--color-foreground` | `#ECEDEF` | texto |
| `--color-muted-foreground` | `#8B909A` | labels, hints |
| `--color-border` | `rgba(255, 255, 255, 0.08)` | líneas |
| `--color-primary` | `#3DDC97` | ahorro, drop, CTA primary |
| `--color-primary-foreground` | `#06120D` | texto sobre primary |
| `--color-warning` | `#E8B44C` | AVIF lento, ratio > 1 |
| `--color-destructive` | `#F2685A` | errores |

Aliases en `:root` (`--bg`, `--mint`, `--line`, `--bar-h`, …) se reescriben para apuntar a estos valores o se eliminan si ningún componente los usa al final de la pasada. No conviven dos paletas distintas.

`--bar-h` pasa a `56px`. Top chrome es `48px` (`--chrome-h`).

### 4.2 Señal vs atmósfera

Primary se usa en: badge de ahorro, borde del drop overlay, `Button variant="primary"`, foco visible, handle activo del compare.

Se elimina: grain (`#app::after`), glow radial mint (`#app::before` mint), rombo del brand, sombras `mint-glow` en iconos, orbe del drop overlay.

Fondo: un radial neutro (blanco 3% desde arriba). Nada de tint signal.

### 4.3 Tipo y ritmo

- Font: IBM Plex Mono 400/500/600 (ya importada).
- 10px / tracking amplio / uppercase: `.label-xs`.
- 12px: toolbar, stats, filename.
- 13px: body, empty subtitle, botones default.
- 18px: empty title por defecto.
- 28px: empty title desde `lg` (≥1024px). El stage es el hero, no el headline.
- Radios: 4px (chips zoom, kbd), 8px (botones, inputs, badges), 12px (Card/stage).
- Motion: `--ease: cubic-bezier(0.22, 1, 0.36, 1)`. Duraciones 150–400ms. `prefers-reduced-motion` ya existe y se mantiene. El icono del empty state **no** flota.

### 4.4 Firma

El divisor del compare (línea + handle con `%`) es el elemento memorable. Handle: círculo/píldora 8px radius, borde primary al 50% de opacidad, sin glow.

## 5. Layout

Padding 8px al viewport (`#app` es el frame). Sin chrome `position: absolute` superpuesto a la imagen.

```
┌──────────────────────────────────────────────┐
│  minipix / playground     ‹  2/5  ›      ✕  │  48px
├──────────────────────────────────────────────┤
│                                              │
│              STAGE (flex-1)                  │
│                                              │
│  photo.png   1600×900            1×  2×  4× │  footer 40px
├──────────────────────────────────────────────┤
│  Format  Quality  Lossless  Resize │ stats [Download] │  56px
└──────────────────────────────────────────────┘
```

### 5.1 Top chrome

- Izquierda: wordmark `minipix` (sin rombo) + `/ playground` en muted, hidden < `sm`.
- Centro: stepper `‹ n / total ›` **solo si** `jobs.length > 1`. Mismos botones `icon` y atajos flechas.
- Derecha: close (`X` Lucide) **solo si** hay imagen activa. Esc sigue llamando `removeActive`.
- Sin imagen: solo wordmark.

### 5.2 Stage

- `flex-1 min-h-0`. Card: borde, `bg-card`, radius 12px, overflow hidden.
- Checker (compare y preview) cubre el Card. Tamaño de celda 20px, contraste `rgba(255,255,255,0.035)` sobre background.
- Imagen: `max-w-full max-h-full object-contain`. Zoom sigue siendo `transform: scale(n)` en Compare.
- Empty: el Card es un `<div>` clickable (`onclick` + `role="button"` + `tabindex="0"` + Enter/Space), no un `<button>` (el CTA interno sí lo es; anidar botones está prohibido). Título, subtítulo, CTA `Select files` + kbd, chips `Badge outline`. Hint paste (`⌘V` / `Ctrl+V`) en el footer del stage, no absoluto sobre el toolbar.
- Working: original al 40% de opacidad + spinner. Copy en toolbar: `decoding…` / `compressing…`.
- Error: icono Lucide `TriangleAlert`, `errorLabel`, mensaje. Centro del Card.
- Overlay de drop (drag de archivos): inset del stage, borde dashed primary, copy “Drop to compress”. Sin orbe.

### 5.3 Footer del stage (solo con imagen activa)

Una fila 40px dentro del Card (borde superior `border`):

- Izquierda: filename (ellipsis) · `width × height` si hay result.
- Warning AVIF: icono Lucide `Clock` + tooltip “AVIF encode is slow in-browser”. No empuja el filename con una frase larga.
- Derecha: label Zoom + `Segmented` 1× 2× 4×.

### 5.4 Toolbar

Una fila, `min-height: 56px`, `bg-card` sólido, borde superior. Sin backdrop-blur.

Izquierda, `flex-nowrap`, grupos separados por `Separator` vertical 22px:

1. `label-xs` Format + `Select` (aria-label `Output format` — **no cambiar**, lo usa E2E).
2. Smaller + `Slider` + Sharper + número tabular si quality activa. Opacity 0.4 si lossless.
3. `Switch` Lossless.
4. Si hay activa: `Switch` Resize + input px (mismos bordes que Select, focus ring primary).

Derecha, `shrink-0`:

- working: spinner + phase copy.
- error: `errorCode` en destructive.
- done: Original / Output bytes + `Badge` `data-testid="savings"` + `Button primary` Download (icono Lucide `Download`).

`<900px`: los dos grupos apilan (controles arriba, stats+download abajo). El slider puede encogerse (`w-[132px]` → `w-[96px]`). No se oculta Download.

### 5.5 Comportamiento que no cambia

- `globalOptions`, debounce de recompress, WebP fuerza lossless, JPEG/AVIF apagan lossless.
- Pool de workers, `avifSlowWarning` a >2MB con format avif.
- Paste, drag-and-drop, file input hidden, `ACCEPT` MIME.
- Compare: pointer capture, teclado left/right sobre el slider, labels Original / minipix.

## 6. Componentes

Regla: las pantallas (`App`, `EmptyState`, `Viewer`, `Compare`, `Toolbar`) no inventan botones, chips ni iconos. `style=` solo para valores dinámicos (`clip-path`, `scale`, `left` del divisor).

| Componente | Cambio |
| --- | --- |
| `Button` | Variantes `primary` / `secondary` / `ghost` / `icon` sobre tokens. Alturas 32px default, 24px sm, 32px icon. Radius 8. |
| `Select` / `Slider` / `Switch` / `Separator` | Misma API pública. Alturas y radios de la escala. |
| `Badge` | `default` = primary (ahorro). `warning` = `--color-warning`. `outline` = chips de formato. |
| `Card` | Stage y dropzone. |
| `Tooltip` | Solo el icono de warning AVIF. Texto: `AVIF encode is slow in-browser`. |
| `Kbd` | `.kbd` existente, radios 4px, tokens nuevos. |
| `Segmented` (**nuevo**) | `playground/src/lib/components/ui/segmented/`. Props: `value`, `items: {value, label}[]`, `onValueChange`. Usado por zoom. |

Iconos Lucide (tamaño 14–16 en toolbar, 20 en error, 32 en empty): `Download`, `X`, `ChevronLeft`, `ChevronRight`, `Upload`, `Layers`, `TriangleAlert`, `Clock`, `LoaderCircle`.

Copy (inglés, sentence case, igual que hoy):

- Empty title: `Compress images, privately`
- Empty body: `Drop a PNG, JPEG, WebP or AVIF. Everything runs in your browser — nothing is ever uploaded.`
- CTA: `Select files`
- Drop overlay: `Drop to compress`
- Download, Original, Output, Smaller, Sharper, Lossless, Resize, Format, Zoom

Errores: `errorLabel(code)` + `errorMessage`. Sin “sorry”.

## 7. Archivos

Solo `playground/src/**` y, si un selector E2E se rompe, `playground/e2e/smoke.spec.ts`.

- Modify: `playground/src/app.css`
- Modify: `playground/src/App.svelte`
- Modify: `playground/src/lib/components/EmptyState.svelte`
- Modify: `playground/src/lib/components/Viewer.svelte`
- Modify: `playground/src/lib/components/Compare.svelte`
- Modify: `playground/src/lib/components/Toolbar.svelte`
- Modify: `playground/src/lib/components/ui/{button,badge,select,slider,switch,separator,card,tooltip}/*`
- Create: `playground/src/lib/components/ui/segmented/{segmented.svelte,index.ts}`
- Touch E2E only if `getByLabel('Output format')`, `getByRole('button', { name: 'Download' })`, `img[alt="Compressed"]` o `[data-testid="savings"]` dejan de existir.

No: `crates/**`, `playground/src/lib/worker/**`, `playground/src/lib/state.svelte.ts`, workflows de deploy.

## 8. Tests

- `playground` vitest (`pool.test.ts`): no se toca.
- E2E: los tres tests de `smoke.spec.ts` deben seguir verdes sin cambiar aserciones de bytes/goldens.
- `npm run check` (svelte-check) y `npm run lint` en `playground/` verdes.
- Verificación visual: empty, drop overlay, working, done+compare, error, >1 imagen (stepper), resize on, lossless, viewport 1280 y 375.

`e2e/_shots.spec.ts` no es criterio de merge. Si el reveal de 900ms desaparece, bajar ese `waitForTimeout` para que los shots no esperen una animación que ya no corre. Selectores (`img[alt="Compressed"]`, `Output format`) se mantienen.

## 9. Deploy

- Build: el mismo `crates/wasm` → `playground/src/lib/wasm` + `cd playground && npm run build`.
- Host canónico: Cloudflare Pages vía `.github/workflows/deploy-playground.yml`. **No modificar.**
- Vercel: un proyecto estático que sirva `playground/dist` funcionaría; queda documentado como opción, no como trabajo de esta pasada. No `vercel.json`, no rewrite a Next.js.

## 10. Criterio de hecho

1. Tokens de §4 son la única paleta; no queda mint glow, grain ni rombo.
2. Shell de §5 en empty y con imagen, desktop y ~375px.
3. Controles de §6; Lucide en lugar de SVG sueltos en pantallas.
4. `data-testid="savings"`, Download, Output format y `img[alt="Compressed"]` intactos.
5. E2E + `npm run check` verdes.
6. Ningún cambio de bytes de salida.

## 11. Fuera de alcance

Core Rust, bindings, wasm-opt, goldens, publicar npm/PyPI, `@minipix/wasm`, filmstrip, batch download, `effort` en UI, light mode, landing/docs, Next.js, SvelteKit, COOP/COEP, threads WASM.

## 12. Relación con el spec v1

No altera `docs/superpowers/specs/2026-06-11-minipix-design.md`. El playground sigue siendo M2, client-side, Vite + Svelte, host estático. Esta pasada es presentación, no alcance de producto.
