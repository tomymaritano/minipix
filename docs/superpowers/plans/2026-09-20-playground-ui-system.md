# Playground UI system Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unificar el sistema visual del playground Svelte (tokens, shell de tres bandas, primitivas + Segmented, Lucide) sin tocar el motor de compresión.

**Architecture:** Una paleta en `@theme` (`playground/src/app.css`). Las pantallas solo usan `$lib/components/ui`. El shell es flex column (chrome 48px / stage flex-1 / toolbar 56px) con padding 8px. Compare, workers y goldens no cambian de comportamiento.

**Tech Stack:** Vite 8, Svelte 5, Tailwind v4, bits-ui, @lucide/svelte, vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-09-20-playground-ui-system-design.md`

## Global Constraints

- Stack: Vite 8 + Svelte 5 + Tailwind v4 + bits-ui. Sin Next.js, sin SvelteKit.
- Flujo: Empty → drop/paste/⌘O → compare → download. Atajos actuales.
- Tokens únicos en `@theme`. Primary `#3DDC97` = señal, no atmósfera. Sin grain, glow mint, ni rombo.
- Tipo: IBM Plex Mono. Escala 10 / 12 / 13 / 18 / 28.
- Radio 4 / 8 / 12. Grilla 4px. Top 48px, toolbar 56px, viewport padding 8px.
- Selectores E2E intocables: `aria-label="Output format"`, `button` name `Download`, `img[alt="Compressed"]`, `[data-testid="savings"]`.
- No modificar: `crates/**`, `playground/src/lib/worker/**`, `playground/src/lib/state.svelte.ts`, `.github/workflows/deploy-playground.yml`.
- Copy en inglés, sentence case, textos del spec §6.
- `style=` solo para valores dinámicos (`clip-path`, `scale`, `left` del divisor).
- `npm run check` y `npm run lint` en `playground/` deben quedar verdes. E2E `smoke.spec.ts` verdes sin cambiar aserciones de bytes.

## File map

| File | Responsibility |
| --- | --- |
| `playground/src/app.css` | Tokens `@theme` + aliases `:root` + `.kbd` + `.label-xs` + `.checker`. Sin grain/glow. |
| `playground/src/theme.test.ts` | Contrato de paleta (hex, sin feTurbulence/mint-glow, `--bar-h: 56px`). |
| `playground/src/lib/components/ui/button/button.svelte` | Variantes primary/secondary/ghost/icon, h-32/24. |
| `playground/src/lib/components/ui/badge/badge.svelte` | default=primary, warning, outline. |
| `playground/src/lib/components/ui/select/select.svelte` | Trigger 32px, Lucide chevron, tokens. |
| `playground/src/lib/components/ui/slider/slider.svelte` | Track primary, sin glow, `max-[899px]:w-[96px]`. |
| `playground/src/lib/components/ui/switch/switch.svelte` | Primary on, sin mint-glow. |
| `playground/src/lib/components/ui/separator/separator.svelte` | `bg-border`. |
| `playground/src/lib/components/ui/card/card.svelte` | Stage chrome, `rounded-xl`. |
| `playground/src/lib/components/ui/tooltip/tooltip.svelte` | Tokens nuevos. |
| `playground/src/lib/components/ui/segmented/*` | Zoom 1×/2×/4×. |
| `playground/src/App.svelte` | Shell: header + stage slot + overlay + toolbar. |
| `playground/src/lib/components/EmptyState.svelte` | Dropzone = Card del stage. |
| `playground/src/lib/components/Viewer.svelte` | Stage + footer (filename, AVIF tooltip, Segmented). Stepper/close salen de aquí. |
| `playground/src/lib/components/Compare.svelte` | Split; handle = firma; sin glow. |
| `playground/src/lib/components/Toolbar.svelte` | In-flow, no `fixed`. |
| `playground/e2e/smoke.spec.ts` | Añadir test de zoom; no tocar goldens. |
| `playground/e2e/_shots.spec.ts` | Bajar wait del empty shot. |

---

### Task 1: Design tokens

**Files:**

- Create: `playground/src/theme.test.ts`
- Modify: `playground/src/app.css`

**Interfaces:**

- Consumes: nothing
- Produces: `@theme` colors (`background` `#0E1013`, `card` `#16191E`, `secondary` `#1C2026`, `foreground` `#ECEDEF`, `muted-foreground` `#8B909A`, `border` `rgba(255, 255, 255, 0.08)`, `primary` `#3DDC97`, `primary-foreground` `#06120D`, `warning` `#E8B44C`, `destructive` `#F2685A`); `:root` `--bar-h: 56px`; `--chrome-h: 48px`; `--ease`; `.checker`; `.kbd`; `.label-xs`. Clases Tailwind `bg-background`, `bg-card`, `bg-primary`, `text-warning` disponibles.

- [ ] **Step 1: Write the failing theme test**

Create `playground/src/theme.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'app.css'), 'utf8');

describe('app.css tokens', () => {
  test('uses the spec palette', () => {
    expect(css).toContain('#0E1013');
    expect(css).toContain('#16191E');
    expect(css).toContain('#1C2026');
    expect(css).toContain('#ECEDEF');
    expect(css).toContain('#8B909A');
    expect(css).toContain('#3DDC97');
    expect(css).toContain('#06120D');
    expect(css).toContain('#E8B44C');
    expect(css).toContain('#F2685A');
  });

  test('toolbar and chrome heights', () => {
    expect(css).toMatch(/--bar-h:\s*56px/);
    expect(css).toMatch(/--chrome-h:\s*48px/);
  });

  test('drops grain and mint glow atmosphere', () => {
    expect(css).not.toMatch(/feTurbulence/);
    expect(css).not.toContain('mint-glow');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd playground && npx vitest run src/theme.test.ts`

Expected: FAIL (`#0E1013` not found and/or `feTurbulence` / `mint-glow` still present).

- [ ] **Step 3: Replace `playground/src/app.css` with the spec tokens**

Overwrite `playground/src/app.css` with:

```css
@import '@fontsource/ibm-plex-mono/400.css';
@import '@fontsource/ibm-plex-mono/500.css';
@import '@fontsource/ibm-plex-mono/600.css';
@import 'tailwindcss';

@theme {
  --font-sans: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Code', 'Menlo', monospace;
  --font-mono: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Code', 'Menlo', monospace;

  --color-background: #0e1013;
  --color-foreground: #ecedef;

  --color-card: #16191e;
  --color-card-foreground: #ecedef;

  --color-primary: #3ddc97;
  --color-primary-foreground: #06120d;

  --color-secondary: #1c2026;
  --color-secondary-foreground: #ecedef;

  --color-muted: #1c2026;
  --color-muted-foreground: #8b909a;

  --color-accent: #3ddc97;
  --color-accent-foreground: #06120d;

  --color-destructive: #f2685a;
  --color-destructive-foreground: #ecedef;

  --color-warning: #e8b44c;
  --color-border: rgba(255, 255, 255, 0.08);
  --color-input: rgba(255, 255, 255, 0.08);
  --color-ring: #3ddc97;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 8px;
  --radius-xl: 12px;
  --radius: 8px;
}

:root {
  --bg: var(--color-background);
  --bg-1: var(--color-card);
  --bg-2: var(--color-card);
  --bg-3: var(--color-secondary);
  --fg: var(--color-foreground);
  --fg-dim: var(--color-muted-foreground);
  --fg-faint: var(--color-muted-foreground);
  --line: var(--color-border);
  --mint: var(--color-primary);
  --warn: var(--color-warning);
  --bad: var(--color-destructive);
  --bar-h: 56px;
  --chrome-h: 48px;
  --ease: cubic-bezier(0.22, 1, 0.36, 1);
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.4);
  --shadow-md: 0 4px 16px -2px rgba(0, 0, 0, 0.5);
  --shadow-lg: 0 12px 40px -8px rgba(0, 0, 0, 0.6);
}

html,
body {
  height: 100%;
}

body {
  background: var(--color-background);
  color: var(--color-foreground);
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.45;
  font-feature-settings: 'zero';
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
  overflow: hidden;
}

img,
picture,
canvas,
svg {
  display: block;
  max-width: 100%;
}

#app {
  height: 100vh;
  height: 100dvh;
  position: relative;
  isolation: isolate;
}

#app::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  background: radial-gradient(90% 60% at 50% -8%, rgba(255, 255, 255, 0.03), transparent 55%);
}

button {
  font: inherit;
  color: inherit;
  background: none;
  border: none;
  cursor: pointer;
}

input,
select {
  font: inherit;
  color: inherit;
}

::selection {
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  color: var(--color-foreground);
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  padding: 2px 6px;
  font-size: 11px;
  font-weight: 500;
  line-height: 1;
  color: var(--color-muted-foreground);
  background: var(--color-secondary);
  border: 1px solid var(--color-border);
  border-radius: 4px;
  box-shadow:
    0 1px 0 rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

.label-xs {
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--color-muted-foreground);
}

.checker {
  background-color: var(--color-background);
  background-image:
    linear-gradient(45deg, rgba(255, 255, 255, 0.035) 25%, transparent 25%),
    linear-gradient(-45deg, rgba(255, 255, 255, 0.035) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgba(255, 255, 255, 0.035) 75%),
    linear-gradient(-45deg, transparent 75%, rgba(255, 255, 255, 0.035) 75%);
  background-size: 20px 20px;
  background-position:
    0 0,
    0 10px,
    10px -10px,
    -10px 0;
}

*::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
*::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--color-foreground) 20%, transparent);
  border-radius: 8px;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    transition-duration: 0.001ms !important;
  }
}
```

Hex in the file may be lowercased by prettier; the test uses the same digits. If prettier uppercases, keep the test case-insensitive (`toMatch(/#0e1013/i)`). Prefer updating the test to `/i` if prettier rewrites case.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd playground && npx vitest run src/theme.test.ts`

Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add playground/src/app.css playground/src/theme.test.ts
git commit -m "feat(playground): unify design tokens to spec palette"
```

---

### Task 2: Restyle UI primitives

**Files:**

- Create: `playground/src/lib/components/ui/primitives.test.ts`
- Modify: `playground/src/lib/components/ui/button/button.svelte`
- Modify: `playground/src/lib/components/ui/badge/badge.svelte`
- Modify: `playground/src/lib/components/ui/select/select.svelte`
- Modify: `playground/src/lib/components/ui/slider/slider.svelte`
- Modify: `playground/src/lib/components/ui/switch/switch.svelte`
- Modify: `playground/src/lib/components/ui/separator/separator.svelte`
- Modify: `playground/src/lib/components/ui/card/card.svelte`
- Modify: `playground/src/lib/components/ui/tooltip/tooltip.svelte`

**Interfaces:**

- Consumes: tokens from Task 1 (`bg-primary`, `text-warning`, `bg-card`, `border-border`, `bg-secondary`, `text-destructive`, `ring-ring`)
- Produces: same public APIs as today (`Button` variants `default | primary | ghost | secondary | icon`, sizes `default | sm | icon`; `Badge` variants `default | warning | outline`; `Select` `aria-label` + `items` + `onValueChange`; `Slider` `value/min/max/onValueChange`; `Switch` `checked/label/onCheckedChange`; `Separator` `orientation`; `Card`; `Tooltip` `content` + `children`). `default` Button looks like `secondary` (raised dark, no white ink pill). Heights: Button default/icon 32px, sm 24px. Select trigger 32px. No `mint-glow`. Select chevron/check use Lucide (`ChevronDown`, `Check`).

- [ ] **Step 1: Write the failing primitives contract test**

Create `playground/src/lib/components/ui/primitives.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const dir = dirname(fileURLToPath(import.meta.url));
const read = (rel: string): string => readFileSync(join(dir, rel), 'utf8');

describe('ui primitives', () => {
  test('do not use mint-glow', () => {
    for (const rel of [
      'button/button.svelte',
      'badge/badge.svelte',
      'select/select.svelte',
      'slider/slider.svelte',
      'switch/switch.svelte',
    ]) {
      expect(read(rel), rel).not.toContain('mint-glow');
    }
  });

  test('signal color goes through the primary token class', () => {
    expect(read('button/button.svelte')).toContain('bg-primary');
    expect(read('badge/badge.svelte')).toContain('text-primary');
    expect(read('slider/slider.svelte')).toContain('bg-primary');
    expect(read('switch/switch.svelte')).toContain('bg-primary');
  });

  test('select keeps the e2e aria-label forwarding', () => {
    expect(read('select/select.svelte')).toContain("'aria-label'");
    expect(read('select/select.svelte')).toContain('aria-label={ariaLabel}');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd playground && npx vitest run src/lib/components/ui/primitives.test.ts`

Expected: FAIL (`mint-glow` still in switch/slider/button, `bg-primary` missing).

- [ ] **Step 3: Rewrite each primitive**

`button.svelte` — keep the same Props interface. Replace `variants`/`sizes`:

```ts
  const base =
    'relative inline-flex items-center justify-center font-medium select-none transition-[transform,background-color,border-color,box-shadow,color] duration-150 ease-[var(--ease)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none cursor-pointer';

  const variants: Record<string, string> = {
    default:
      'bg-secondary text-foreground border border-border hover:border-foreground/20',
    primary: 'bg-primary text-primary-foreground hover:brightness-110',
    ghost: 'bg-transparent text-muted-foreground hover:text-foreground hover:bg-white/[0.06]',
    secondary: 'bg-secondary text-foreground border border-border hover:border-foreground/20',
    icon: 'bg-transparent text-muted-foreground hover:text-foreground hover:bg-white/[0.06]',
  };

  const sizes: Record<string, string> = {
    default: 'gap-2 px-3.5 h-8 text-[13px] rounded-md',
    sm: 'gap-1 px-2 h-6 text-[11px] rounded-md',
    icon: 'h-8 w-8 rounded-md',
  };
```

`badge.svelte`:

```ts
  const base =
    'inline-flex items-center justify-center font-semibold tabular-nums text-[11px] leading-none rounded-full border px-2 py-0.5 transition-colors duration-150';

  const variants: Record<string, string> = {
    default: 'text-primary bg-primary/16 border-primary/30',
    warning: 'text-warning bg-warning/16 border-warning/30',
    outline: 'text-muted-foreground bg-transparent border-border',
  };
```

`separator.svelte`: class `shrink-0 bg-border` (drop `bg-[var(--line)]`).

`card.svelte`: `'rounded-xl border border-border bg-card text-foreground shadow-[var(--shadow-md)]'`

`tooltip.svelte`: content class `'z-[80] rounded-md border border-border bg-secondary px-2 py-1 text-[11px] font-medium text-foreground shadow-[var(--shadow-lg)] tt-content'`. Keep bits-ui structure and `tt-in` animation.

`switch.svelte`: Root checked `'bg-primary border-primary'` (no glow). Unchecked `'bg-secondary border-border'`. Thumb checked `'translate-x-[14px] bg-background'`. Focus `focus-visible:ring-ring focus-visible:ring-offset-background`. Label checked `text-foreground`, else `text-muted-foreground`. Sizes stay 16×28 / thumb 11.

`slider.svelte`: Root add `'w-[132px] max-[899px]:w-[96px]'`. Track fill `'absolute inset-y-0 left-0 rounded-full bg-primary'` (no glow). Track base `'bg-secondary'`. Thumb `'h-3.5 w-3.5 rounded-full border border-border bg-foreground'`. Keep `style:width` for the fill (dynamic). Keep `handleChange` as-is.

`select.svelte`: Replace inline chevron SVG with `import { Check, ChevronDown } from '@lucide/svelte'`. Trigger classes:

```
'select-trigger inline-flex h-8 items-center gap-2 rounded-md border border-border bg-secondary px-2.5 text-[12px] font-medium text-foreground transition-colors duration-150 hover:border-foreground/20 data-[state=open]:border-primary data-[state=open]:ring-2 data-[state=open]:ring-primary/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer'
```

Content: `'z-[80] min-w-[var(--bits-select-anchor-width,96px)] overflow-hidden rounded-md border border-border bg-secondary p-1 shadow-[var(--shadow-lg)] select-content'`

Item selected: `'data-[selected]:text-primary'`. Use `<ChevronDown size={12} class="select-chevron ..." />` and `<Check size={12} />` instead of path SVGs. Keep `aria-label={ariaLabel}` on Trigger. Keep open-state chevron rotate CSS; color on open: `color: var(--color-primary)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd playground && npx vitest run src/lib/components/ui/primitives.test.ts src/theme.test.ts`

Expected: PASS.

Also run: `cd playground && npm run check`

Expected: exit 0. If bits-ui types complain, fix without changing public props.

- [ ] **Step 5: Commit**

```bash
git add playground/src/lib/components/ui
git commit -m "feat(playground): restyle ui primitives onto spec tokens"
```

---

### Task 3: Segmented control

**Files:**

- Create: `playground/src/lib/components/ui/segmented/segmented.svelte`
- Create: `playground/src/lib/components/ui/segmented/index.ts`
- Create: `playground/src/lib/components/ui/segmented/segmented.test.ts`

**Interfaces:**

- Consumes: `Button` not required; raw buttons inside a labelled group. Tokens from Task 1.
- Produces:

```ts
export interface SegmentedItem {
  value: string;
  label: string;
}

export interface SegmentedProps {
  value: string;
  items: SegmentedItem[];
  onValueChange: (value: string) => void;
  'aria-label'?: string;
  class?: string;
}
```

Export: `export { default as Segmented } from './segmented.svelte'` in `index.ts`.

Each item is `<button type="button" aria-pressed={selected}>`. Selected: `bg-secondary text-foreground shadow-[var(--shadow-sm)]`. Unselected: `text-muted-foreground hover:text-foreground`. Group: `inline-flex gap-0.5 rounded-md border border-border bg-card p-0.5`. Item size `h-[22px] min-w-[30px] rounded-sm text-[11px] font-medium tabular-nums`.

- [ ] **Step 1: Write the failing contract test**

Create `playground/src/lib/components/ui/segmented/segmented.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const src = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), 'segmented.svelte'),
  'utf8',
);

describe('Segmented', () => {
  test('exposes pressed state and value callback', () => {
    expect(src).toContain('aria-pressed');
    expect(src).toContain('onValueChange');
    expect(src).toContain('aria-label');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd playground && npx vitest run src/lib/components/ui/segmented/segmented.test.ts`

Expected: FAIL (ENOENT segmented.svelte).

- [ ] **Step 3: Implement Segmented**

`segmented.svelte`:

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';

  interface SegmentedItem {
    value: string;
    label: string;
  }

  interface Props {
    value: string;
    items: SegmentedItem[];
    onValueChange: (value: string) => void;
    'aria-label'?: string;
    class?: string;
  }

  const {
    value,
    items,
    onValueChange,
    'aria-label': ariaLabel = 'Zoom',
    class: className,
  }: Props = $props();
</script>

<div
  role="group"
  aria-label={ariaLabel}
  class={cn(
    'inline-flex items-center gap-0.5 rounded-md border border-border bg-card p-0.5',
    className,
  )}
>
  {#each items as item (item.value)}
    <button
      type="button"
      aria-pressed={value === item.value}
      aria-label={item.label}
      onclick={() => onValueChange(item.value)}
      class={cn(
        'h-[22px] min-w-[30px] rounded-sm px-1.5 text-[11px] font-medium tabular-nums transition-colors duration-150',
        value === item.value
          ? 'bg-secondary text-foreground shadow-[var(--shadow-sm)]'
          : 'text-muted-foreground hover:text-foreground',
      )}
    >
      {item.label}
    </button>
  {/each}
</div>
```

`index.ts`:

```ts
export { default as Segmented } from './segmented.svelte';
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd playground && npx vitest run src/lib/components/ui/segmented/segmented.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add playground/src/lib/components/ui/segmented
git commit -m "feat(playground): add Segmented control for zoom"
```

---

### Task 4: App shell + empty state

**Files:**

- Modify: `playground/src/App.svelte`
- Modify: `playground/src/lib/components/EmptyState.svelte`
- Modify: `playground/e2e/_shots.spec.ts` (empty wait)

**Interfaces:**

- Consumes: `Button`, `Badge`, `Card` (optional — EmptyState may use Card or equivalent classes), Lucide `X`, `ChevronLeft`, `ChevronRight`, `Layers`.
- Produces: `#app` child is `flex h-full flex-col p-2`. Header `h-12` (`--chrome-h`) with wordmark `minipix` + `/ playground` (hidden `< sm`), stepper centered only when `playground.position.total > 1`, close (`aria-label="Close"`) only when `playground.active`. Stage wrapper `relative min-h-0 flex-1`. Toolbar in-flow (not yet restyled; still imported). Drop overlay covers the stage wrapper only (not the chrome/toolbar): dashed `border-primary`, copy `Drop to compress`, no orb. EmptyState fills the stage: clickable `<div>` (NO `role="button"` — inner CTA is the button; wrapper `onclick` + Enter not required on wrapper). Paste hint inside the card bottom. Title `text-[18px] lg:text-[28px]`. No float animation on the icon. `⌘O` kbd on the CTA. File input, paste, drag, keyboard handlers stay in App.

- [ ] **Step 1: Write the failing empty-shot timeout change as a comment-driven check, then a source assertion test**

Create `playground/src/shell.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const here = dirname(fileURLToPath(import.meta.url));
const app = readFileSync(join(here, 'App.svelte'), 'utf8');
const empty = readFileSync(join(here, 'lib/components/EmptyState.svelte'), 'utf8');

test('app shell is in-flow flex, not overlay brand', () => {
  expect(app).toContain('flex h-full flex-col p-2');
  expect(app).not.toMatch(/◆/);
});

test('empty state is a div dropzone with an inner Button', () => {
  expect(empty).toContain('Select files');
  expect(empty).not.toMatch(/icon-float/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd playground && npx vitest run src/shell.test.ts`

Expected: FAIL (`flex h-full flex-col p-2` missing; `◆` still in App).

- [ ] **Step 3: Implement App shell and EmptyState**

Keep all existing App `<script>` logic (file input, drag depth, paste, keydown). Change the markup after the hidden file input to:

```svelte
<div class="relative z-[2] flex h-full flex-col p-2">
  <header class="relative flex h-12 shrink-0 items-center px-3">
    <div class="flex select-none items-center gap-2 text-[13px] tracking-tight">
      <span class="font-medium text-foreground">minipix</span>
      <span class="hidden text-[10px] tracking-[0.04em] text-muted-foreground sm:inline"
        >/ playground</span
      >
    </div>

    {#if playground.position && playground.position.total > 1}
      <div
        class="absolute left-1/2 inline-flex -translate-x-1/2 items-center gap-2.5 text-[12px] tabular-nums text-muted-foreground"
      >
        <Button
          variant="icon"
          size="sm"
          class="h-6 w-6 rounded-full px-0"
          onclick={() => playground.step(-1)}
          aria-label="Previous image"
        >
          <ChevronLeft size={16} />
        </Button>
        <span class="min-w-[42px] text-center font-medium text-foreground">
          {playground.position.index}
          <span class="text-muted-foreground">/</span>
          {playground.position.total}
        </span>
        <Button
          variant="icon"
          size="sm"
          class="h-6 w-6 rounded-full px-0"
          onclick={() => playground.step(1)}
          aria-label="Next image"
        >
          <ChevronRight size={16} />
        </Button>
      </div>
    {/if}

    {#if playground.active}
      <Button
        variant="icon"
        size="icon"
        class="ml-auto"
        onclick={() => playground.removeActive()}
        aria-label="Close"
      >
        <X size={15} />
      </Button>
    {/if}
  </header>

  <div class="relative min-h-0 flex-1">
    {#if playground.active}
      <Viewer job={playground.active} />
    {:else}
      <EmptyState {openPicker} {isMac} />
    {/if}

    {#if dragOver}
      <div
        class="drop-overlay pointer-events-none absolute inset-0 z-20 grid place-items-center rounded-xl border-2 border-dashed border-primary bg-background/70"
      >
        <span class="text-[16px] font-medium tracking-[0.01em] text-primary">Drop to compress</span>
      </div>
    {/if}
  </div>

  <Toolbar active={playground.active} />
</div>
```

Imports: `ChevronLeft`, `ChevronRight`, `X` from `@lucide/svelte`; `Button` from `$lib/components/ui/button`. Keep the `pop` keyframes for `.drop-overlay` in the App `<style>` (150ms). Remove the old absolute brand header.

`EmptyState.svelte` markup (script keeps `openPicker`, `isMac`, `mod`, `formats`):

```svelte
<div class="flex h-full min-h-0 flex-col">
  <div
    class="group relative flex min-h-0 flex-1 cursor-pointer flex-col items-center justify-center rounded-xl border border-dashed border-border bg-card px-10 py-10 text-center hover:border-primary/55"
    onclick={openPicker}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        openPicker();
      }
    }}
  >
    <div
      class="mb-7 grid h-[84px] w-[84px] place-items-center rounded-xl border border-border bg-secondary"
    >
      <Layers size={40} class="text-primary" />
    </div>

    <h1 class="text-[18px] font-medium leading-[1.1] tracking-[-0.02em] text-foreground lg:text-[28px]">
      Compress images, privately
    </h1>
    <p class="mt-3 max-w-[400px] text-[13px] leading-relaxed text-muted-foreground">
      Drop a PNG, JPEG, WebP or AVIF. Everything runs in your browser —
      <span class="text-foreground">nothing is ever uploaded.</span>
    </p>

    <div class="mt-8">
      <Button
        variant="primary"
        class="h-10 px-6 text-[13px]"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          openPicker();
        }}
      >
        Select files
        <span
          class="kbd ml-1 border-primary-foreground/25 bg-primary-foreground/15 text-primary-foreground shadow-none"
          >{mod}O</span
        >
      </Button>
    </div>

    <div class="mt-7 flex items-center gap-1.5">
      {#each formats as f, i (f)}
        {#if i > 0}
          <span class="text-muted-foreground/40">·</span>
        {/if}
        <Badge variant="outline" class="px-2 py-1 text-[10px] tracking-[0.06em]">{f}</Badge>
      {/each}
    </div>

    <p
      class="absolute bottom-4 inline-flex items-center gap-1.5 text-[11px] text-muted-foreground"
    >
      <span class="kbd">{mod}V</span>
      <span>to paste from clipboard</span>
    </p>
  </div>
</div>
```

Do **not** put `role="button"` on the wrapper (nested with the CTA). The wrapper is a mouse drop/click target; the CTA is the accessible control. `aria-label` on the CTA Button is enough (`Select files`).

Remove all `rise` / `icon-float` / `reveal-*` animations from EmptyState.

In `e2e/_shots.spec.ts`, change the empty shot wait from `900` to `200`:

```ts
  await page.waitForTimeout(200);
```

- [ ] **Step 4: Run tests**

Run: `cd playground && npx vitest run src/shell.test.ts src/theme.test.ts`

Expected: PASS.

Run: `cd playground && npm run check`

Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add playground/src/App.svelte playground/src/lib/components/EmptyState.svelte playground/src/shell.test.ts playground/e2e/_shots.spec.ts
git commit -m "feat(playground): three-band shell and stage empty state"
```

---

### Task 5: Viewer + Compare

**Files:**

- Modify: `playground/src/lib/components/Viewer.svelte`
- Modify: `playground/src/lib/components/Compare.svelte`
- Modify: `playground/e2e/smoke.spec.ts`

**Interfaces:**

- Consumes: `Segmented` (`value` string `'1' | '2' | '4'`, items labels `1×` `2×` `4×`, `aria-label="Zoom"`), `Tooltip` (`content="AVIF encode is slow in-browser"`), Lucide `TriangleAlert`, `Clock`, `LoaderCircle`. `.checker` from Task 1.
- Produces: Viewer is `flex h-full min-h-0 flex-col overflow-hidden rounded-xl border border-border bg-card` — **no** close, **no** stepper (those live in App). Stage `flex-1 min-h-0` with no max-h 640 / max-w 920. Footer `h-10` border-t: filename ellipsis, dims `width × height`, AVIF `Clock`+Tooltip (no long sentence), Zoom + Segmented. Compare keeps pointer/keyboard split, `img[alt="Compressed"]` and `img[alt="Original"]`. Handle: pill `rounded-md`, `border-primary/50`, `bg-card`, `%` tabular, no glow. minipix label: `text-primary` without the glowing dot. `style` only for `clip-path`, `transform: scale`, `left` of divider, `image-rendering`.

- [ ] **Step 1: Write the failing zoom E2E**

Append to `playground/e2e/smoke.spec.ts`:

```ts
test('zoom segmented is pressed at 1× then 2×', async ({ page }) => {
  await page.goto('/');
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/gradient_circle.png'));
  await expect(page.locator('img[alt="Compressed"]')).toBeVisible({ timeout: 120_000 });
  const z1 = page.getByRole('button', { name: '1×' });
  const z2 = page.getByRole('button', { name: '2×' });
  await expect(z1).toHaveAttribute('aria-pressed', 'true');
  await z2.click();
  await expect(z2).toHaveAttribute('aria-pressed', 'true');
  await expect(z1).toHaveAttribute('aria-pressed', 'false');
});
```

- [ ] **Step 2: Run E2E zoom test to verify it fails**

WASM must already exist at `playground/src/lib/wasm/` (built earlier in this repo). From `playground/`:

Run: `npx playwright test e2e/smoke.spec.ts -g "zoom segmented"`

Expected: FAIL (timeout or missing `1×` button — current zoom buttons are `1×` text but not `aria-pressed` via Segmented; after Task 4 they may still be the old cluster). If the old buttons already say `1×` without `aria-pressed`, the assertion on `aria-pressed` fails. That is the desired fail.

Note: `npm run e2e` builds first (`vite build` + preview). Prefer that if preview port is the one in `playwright.config.ts` (`4173`).

Run: `cd playground && npm run e2e -- --grep "zoom segmented"`

Expected: FAIL.

- [ ] **Step 3: Implement Viewer and Compare**

Viewer script: keep `job`, `zoom` (`$state(1)`), `beforeUrl`/`afterUrl` effects, `pos`. Add:

```ts
  import { Segmented } from '$lib/components/ui/segmented';
  import { Tooltip } from '$lib/components/ui/tooltip';
  import { TriangleAlert, Clock, LoaderCircle } from '@lucide/svelte';

  const zoomItems = [
    { value: '1', label: '1×' },
    { value: '2', label: '2×' },
    { value: '4', label: '4×' },
  ];
```

Remove the top bar (stepper + close). Structure:

```svelte
<div class="flex h-full min-h-0 flex-col overflow-hidden rounded-xl border border-border bg-card">
  <div class="relative min-h-0 flex-1">
    {#if job.status === 'error'}
      <!-- TriangleAlert 20px, errorLabel, errorMessage. Centered. -->
    {:else if job.result && afterUrl && beforeUrl}
      {#key job.id}
        <Compare {beforeUrl} {afterUrl} {zoom} />
      {/key}
    {:else if beforeUrl}
      <div class="checker relative grid h-full w-full place-items-center p-6">
        <img src={beforeUrl} alt={job.file.name} class="max-h-full max-w-full opacity-40" />
        <div class="absolute inset-0 grid place-items-center">
          <LoaderCircle size={28} class="animate-spin text-primary" />
        </div>
      </div>
    {/if}
  </div>

  <div class="flex h-10 shrink-0 items-center justify-between gap-3 border-t border-border px-4">
    <div class="inline-flex min-w-0 items-center gap-3 text-[12px]">
      <span class="max-w-[40vw] overflow-hidden text-ellipsis whitespace-nowrap font-medium text-foreground"
        >{job.file.name}</span
      >
      {#if job.result}
        <span class="tabular-nums text-muted-foreground"
          >{job.result.width} <span class="opacity-50">×</span> {job.result.height}</span
        >
      {/if}
      {#if job.avifSlowWarning}
        <Tooltip content="AVIF encode is slow in-browser">
          <span class="inline-flex text-warning" aria-label="AVIF encode is slow in-browser">
            <Clock size={14} />
          </span>
        </Tooltip>
      {/if}
    </div>
    {#if job.result}
      <div class="inline-flex items-center gap-2 text-[12px]">
        <span class="label-xs">Zoom</span>
        <Segmented
          value={String(zoom)}
          items={zoomItems}
          onValueChange={(v) => {
            zoom = Number(v);
          }}
        />
      </div>
    {/if}
  </div>
</div>
```

Keep a short fade animation on `.viewer` if you wrap the root with class `viewer`; optional. No stage max-width.

Compare: add `class="checker"` on the root (remove the local `.checker` `<style>` background — use global). Labels:

- Original: `absolute left-4 top-4 z-[4] rounded-md border border-border bg-card/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground`
- minipix: same on the right, `text-primary border-primary/25 bg-primary/10` — **no** inner glowing dot.

Divider handle:

```svelte
  <div
    class="pointer-events-none absolute bottom-0 top-0 z-[5] w-0"
    style:left="{pos}%"
    style="transform: translateX(-0.5px);"
  >
    <div class="absolute bottom-0 left-0 top-0 w-px bg-white/80"></div>
    <div
      class="absolute left-0 top-1/2 grid h-7 min-w-[46px] -translate-x-1/2 -translate-y-1/2 place-items-center rounded-md border border-primary/50 bg-card px-2.5 shadow-[var(--shadow-md)]"
    >
      <span class="text-[11px] font-medium tabular-nums text-foreground">{Math.round(pos)}%</span>
    </div>
  </div>
```

Keep pointer handlers, `role="slider"`, alts `Compressed` / `Original`, dynamic `style:clip-path`, `style:transform`.

- [ ] **Step 4: Run E2E zoom + existing smokes**

Run: `cd playground && npm run e2e`

Expected: all 4 tests PASS (3 original + zoom). Goldens unchanged.

- [ ] **Step 5: Commit**

```bash
git add playground/src/lib/components/Viewer.svelte playground/src/lib/components/Compare.svelte playground/e2e/smoke.spec.ts
git commit -m "feat(playground): stage viewer with Segmented zoom and compare handle"
```

---

### Task 6: Toolbar in-flow

**Files:**

- Modify: `playground/src/lib/components/Toolbar.svelte`

**Interfaces:**

- Consumes: Button, Select, Slider, Switch, Badge, Separator, Lucide `Download`, `LoaderCircle`. Tokens. `aria-label="Output format"` unchanged. `data-testid="savings"` unchanged. Button name `Download` unchanged.
- Produces: Root is **not** `fixed`. `shrink-0 border-t border-border bg-card`, `min-h-[56px]`. Groups as spec §5.4. `<900px` (`max-[899px]:flex-col`) stacks controls above stats. Resize input uses Select-like chrome (`h-8 rounded-md border border-border bg-secondary`, focus `ring-2 ring-primary/30`). No backdrop-blur. Spinner: `LoaderCircle` `animate-spin`. Download: `<Download size={14} /> Download`.

- [ ] **Step 1: Write the failing contract test**

Create `playground/src/lib/components/toolbar.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const src = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'Toolbar.svelte'), 'utf8');

test('toolbar is in-flow and keeps e2e hooks', () => {
  expect(src).not.toMatch(/fixed inset-x-0 bottom-0/);
  expect(src).toContain('data-testid="savings"');
  expect(src).toContain('Output format');
  expect(src).toContain('Download');
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd playground && npx vitest run src/lib/components/toolbar.test.ts`

Expected: FAIL (`fixed inset-x-0 bottom-0` still present).

- [ ] **Step 3: Restyle Toolbar**

Keep the entire `<script>` (recompress, pickFormat, lossless, resize, download). Replace the root wrapper classes:

From:

```
class="toolbar fixed inset-x-0 bottom-0 z-40 border-t border-[var(--line)]"
style="background: linear-gradient(...); backdrop-filter: ..."
```

To:

```
class="toolbar shrink-0 border-t border-border bg-card"
```

Inner:

```
class="mx-auto flex w-full min-h-[56px] max-w-[1520px] flex-col gap-3 px-5 py-3 min-[900px]:flex-row min-[900px]:flex-wrap min-[900px]:items-center min-[900px]:justify-between lg:px-7"
```

Replace working spinner span with:

```svelte
<LoaderCircle size={12} class="animate-spin text-primary" />
```

Error text: `text-destructive`. Stats labels stay `label-xs`. Badge unchanged aside from primitive restyle. Download:

```svelte
<Button variant="primary" onclick={download}>
  <Download size={14} />
  Download
</Button>
```

Resize input:

```
class="w-[52px] bg-transparent px-2 py-1.5 text-[12px] tabular-nums text-foreground outline-none"
```

Wrapper:

```
class="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary pr-2 focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/30"
```

Quality cluster: `style:opacity={qualityActive ? '1' : '0.4'}` stays (dynamic). Number width `w-[26px] text-[11px] tabular-nums text-muted-foreground`.

- [ ] **Step 4: Run tests**

Run: `cd playground && npx vitest run src/lib/components/toolbar.test.ts`

Expected: PASS.

Run: `cd playground && npm run check && npm run lint && npm run e2e`

Expected: all green. If prettier/eslint fail on class wrapping, format with `npm run format` and re-lint. Do not drop `data-testid="savings"` or `aria-label="Output format"`.

- [ ] **Step 5: Commit**

```bash
git add playground/src/lib/components/Toolbar.svelte playground/src/lib/components/toolbar.test.ts
git commit -m "feat(playground): in-flow toolbar on spec tokens"
```

---

### Task 7: Visual + quality gate

**Files:** none new unless a selector slipped.

**Interfaces:**

- Consumes: Tasks 1–6 complete.
- Produces: `npm run check`, `npm run lint`, `npx vitest run`, `npm run e2e` all exit 0. No `mint-glow`, `feTurbulence`, or `◆` in `playground/src`. Manual pass of spec §10 on the running app.

- [ ] **Step 1: Grep leftovers**

Run from repo root:

```bash
rg -n "mint-glow|feTurbulence|◆|icon-float" playground/src
```

Expected: no matches. If any remain in comments or dead CSS, delete them.

- [ ] **Step 2: Unit tests**

Run: `cd playground && npx vitest run`

Expected: PASS (theme, primitives, segmented, shell, toolbar, existing pool tests).

- [ ] **Step 3: Typecheck + lint**

Run: `cd playground && npm run check && npm run lint`

Expected: exit 0.

- [ ] **Step 4: E2E**

Run: `cd playground && npm run e2e`

Expected: 4 tests PASS. SHA goldens identical to before.

- [ ] **Step 5: Browser check (required by spec §8 / §10)**

Dev server: `cd playground && npm run dev` (WASM already in `src/lib/wasm`). Exercise:

1. Empty at 1280 and 375: title, CTA, chips, paste hint, no glow/grain/rombo.
2. Drop overlay: dashed primary, no orb.
3. PNG compress: compare, savings, Download, zoom 1×/2×/4×.
4. Two files: stepper in top chrome, close, Esc.
5. Lossless + Resize on.
6. AVIF + file >2MB if available: Clock tooltip, not a sentence in the footer.

If a gap vs spec, fix in the owning file from Tasks 4–6 and re-run Step 2–4.

- [ ] **Step 6: Commit only if Step 5 produced fixes**

```bash
git add playground/src playground/e2e
git commit -m "fix(playground): close ui system spec gaps"
```

If nothing to commit, skip.

---

## Spec coverage

| Spec section | Task |
| --- | --- |
| §4 tokens, no grain/glow, radial neutro, type, radius | 1 |
| §4.1 aliases, `--bar-h` / `--chrome-h` | 1 |
| §6 Button/Badge/Select/Slider/Switch/Separator/Card/Tooltip | 2 |
| §6 Segmented | 3 |
| §5 shell, top chrome, empty stage, drop overlay, paste hint | 4 |
| §4.3 no icon float; empty title 18/28 | 4 |
| §5.2–5.3 viewer, footer, compare firma §4.4 | 5 |
| §5.4 toolbar, 900px stack | 6 |
| §6 Lucide icons | 4, 5, 6 |
| §8 tests, §10 done criteria | 5 (zoom e2e), 7 |
| §9 deploy / §11 out of scope | no files (not in plan) |
| E2E hooks | 2, 5, 6 |

## Placeholder / type check

- Segmented `value` is `string`; Viewer stores `zoom` as `number` and converts with `String`/`Number`.
- Button `default` === visual `secondary`.
- Empty wrapper is **not** `role="button"` (nested CTA). Spec’s “not a `<button>` wrapping a button” is implemented that way.
- No TBD/TODO in tasks.
