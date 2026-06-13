# shadcn-svelte UI Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-skin the minipix playground UI from bespoke CSS variables to shadcn-svelte (Tailwind v4 + bits-ui) with a dark IBM Plex Mono theme, without touching the compression engine.

**Architecture:** Install Tailwind v4 (CSS-first, no config file) with `@tailwindcss/vite` plugin, set up $lib alias for non-SvelteKit project, manually scaffold shadcn-svelte primitives (clsx, tailwind-merge, bits-ui), then add individual UI components (Button, Select, Slider, Switch, Badge) and restyle all five components keeping all logic intact.

**Tech Stack:** Svelte 5.55, Vite 8, Tailwind v4 (@tailwindcss/vite), bits-ui, clsx, tailwind-merge, tailwind-variants, @lucide/svelte, IBM Plex Mono (@fontsource)

---

## Files Modified/Created

| File                                   | Action | Purpose                                                                           |
| -------------------------------------- | ------ | --------------------------------------------------------------------------------- |
| `vite.config.ts`                       | Modify | Add tailwindcss() plugin + $lib alias                                             |
| `tsconfig.app.json`                    | Modify | Add paths + baseUrl for $lib                                                      |
| `src/app.css`                          | Modify | Replace bespoke vars with Tailwind v4 theme (dark, emerald accent, IBM Plex Mono) |
| `src/lib/utils.ts`                     | Create | cn() helper (clsx + tailwind-merge)                                               |
| `src/lib/components/ui/button/`        | Create | shadcn Button component                                                           |
| `src/lib/components/ui/select/`        | Create | shadcn Select component                                                           |
| `src/lib/components/ui/slider/`        | Create | shadcn Slider component                                                           |
| `src/lib/components/ui/switch/`        | Create | shadcn Switch component                                                           |
| `src/lib/components/ui/badge/`         | Create | shadcn Badge component                                                            |
| `src/App.svelte`                       | Modify | Replace <style> with Tailwind classes                                             |
| `src/lib/components/EmptyState.svelte` | Modify | Replace <style> with Tailwind classes                                             |
| `src/lib/components/Toolbar.svelte`    | Modify | Replace native controls with shadcn components + Tailwind                         |
| `src/lib/components/Viewer.svelte`     | Modify | Replace <style> with Tailwind classes                                             |
| `src/lib/components/Compare.svelte`    | Modify | Replace <style> with Tailwind classes (keep bespoke logic)                        |
| `eslint.config.js`                     | Modify | Add ignores for src/lib/components/ui/\*\*                                        |
| `e2e/smoke.spec.ts`                    | Modify | Replace selectOption() with click-based selectFormat(), add data-testid="savings" |
| `index.html`                           | Modify | Add class="dark" to <html>                                                        |

---

### Task 1: Tailwind v4 + Vite plugin + $lib alias

**Files:**

- Modify: `vite.config.ts`
- Modify: `tsconfig.app.json`

- [ ] **Step 1: Install Tailwind v4 deps**

```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
cd C:\Users\Tomy\Documents\compressor\playground
npm i -D tailwindcss @tailwindcss/vite
```

Expected: exit 0, packages added to devDependencies.

- [ ] **Step 2: Update vite.config.ts**

```ts
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
});
```

- [ ] **Step 3: Add paths to tsconfig.app.json compilerOptions**

Add `"baseUrl": "."` and `"paths": { "$lib": ["./src/lib"], "$lib/*": ["./src/lib/*"] }`.

- [ ] **Step 4: Verify build succeeds**

```powershell
npm run build
```

Expected: exit 0, no TS/Vite errors.

---

### Task 2: Install runtime deps + scaffold shadcn-svelte utils

**Files:**

- Create: `src/lib/utils.ts`

- [ ] **Step 1: Install runtime packages**

```powershell
npm i clsx tailwind-merge tailwind-variants bits-ui @lucide/svelte
```

Expected: exit 0.

- [ ] **Step 2: Create cn() helper**

`src/lib/utils.ts`:

```ts
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

---

### Task 3: Dark theme in app.css (Tailwind v4 CSS-first)

**Files:**

- Modify: `src/app.css`
- Modify: `index.html` (add `class="dark"`)

- [ ] **Step 1: Replace app.css content**

Keep IBM Plex Mono @imports. Replace custom properties block with Tailwind v4 `@import 'tailwindcss'` + CSS theme. Define dark-mode tokens as `:root` defaults (dark-only app). Use emerald as accent, zinc-950 as base.

Key CSS variables to expose for non-Tailwind usage still needed in Compare.svelte and transitions:

- `--ease`: cubic-bezier(0.22, 1, 0.36, 1)
- Tailwind `--color-primary` → emerald-400 (#4ade80)

Full app.css:

```css
@import '@fontsource/ibm-plex-mono/400.css';
@import '@fontsource/ibm-plex-mono/500.css';
@import '@fontsource/ibm-plex-mono/600.css';
@import 'tailwindcss';

@theme {
  --font-sans: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Code', 'Menlo', monospace;
  --font-mono: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Code', 'Menlo', monospace;

  --color-background: #0c0d10;
  --color-foreground: #e9e9ec;
  --color-card: #101216;
  --color-card-foreground: #e9e9ec;
  --color-primary: #4ade80;
  --color-primary-foreground: #0c0d10;
  --color-secondary: #1c1f26;
  --color-secondary-foreground: #e9e9ec;
  --color-muted: #1c1f26;
  --color-muted-foreground: #6b7280;
  --color-accent: #4ade80;
  --color-accent-foreground: #0c0d10;
  --color-destructive: #f2685a;
  --color-border: rgba(255, 255, 255, 0.07);
  --color-input: rgba(255, 255, 255, 0.07);
  --color-ring: #4ade80;

  --radius: 0.375rem;
}

:root {
  /* Legacy tokens for Compare.svelte + animations */
  --bg: #0c0d10;
  --bg-elev: #101216;
  --fg: #e9e9ec;
  --fg-dim: #777b84;
  --fg-faint: #45484f;
  --fg-ghost: #2a2c31;
  --line: rgba(255, 255, 255, 0.07);
  --line-strong: rgba(255, 255, 255, 0.14);
  --dash: rgba(255, 255, 255, 0.09);
  --mint: #4ade80;
  --mint-dim: rgba(74, 222, 128, 0.16);
  --warn: #f2c14e;
  --bad: #f2685a;
  --ink-btn: #f4f4f5;
  --ink-btn-fg: #0c0d10;
  --ease: cubic-bezier(0.22, 1, 0.36, 1);
  --bar-h: 52px;
}

*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html,
body {
  height: 100%;
}

body {
  background: var(--bg);
  color: var(--fg);
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.45;
  font-feature-settings: 'zero';
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
  overflow: hidden;
  background-image: radial-gradient(
    120% 80% at 50% -10%,
    rgba(255, 255, 255, 0.025),
    transparent 60%
  );
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
}

/* dashed inset frame */
#app::before {
  content: '';
  position: fixed;
  inset: 9px;
  border: 1px dashed var(--dash);
  border-radius: 2px;
  pointer-events: none;
  z-index: 50;
}

/* film grain */
#app::after {
  content: '';
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 60;
  opacity: 0.035;
  mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
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
  background: var(--mint-dim);
  color: var(--fg);
}

.kbd {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px 6px;
  font-size: 11px;
  color: var(--fg-dim);
  background: var(--bg-elev);
  border: 1px solid var(--line);
  border-radius: 4px;
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.4);
}

*::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
*::-webkit-scrollbar-thumb {
  background: var(--fg-ghost);
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

- [ ] **Step 2: Add class="dark" to index.html `<html>` tag**

---

### Task 4: shadcn UI components (Button, Select, Slider, Switch, Badge)

**Files:**

- Create: `src/lib/components/ui/button/index.ts` + `button.svelte`
- Create: `src/lib/components/ui/select/index.ts` + `select.svelte` etc.
- Create: `src/lib/components/ui/slider/index.ts` + `slider.svelte`
- Create: `src/lib/components/ui/switch/index.ts` + `switch.svelte`
- Create: `src/lib/components/ui/badge/index.ts` + `badge.svelte`

Try CLI first:

```powershell
npx --yes shadcn-svelte@latest add button select slider switch badge --overwrite
```

If that fails (non-Kit project), write minimal bespoke-but-shadcn-shaped components by hand using bits-ui + cn(). The Toolbar needs:

- `Button`: renders `<button>` with cn() classes, variant prop (default=white, ghost=transparent)
- `Select`: wraps bits-ui `Select.Root/Trigger/Content/Item` — critical: trigger must have `aria-label="Output format"`
- `Slider`: wraps bits-ui `Slider.Root/Range/Thumb`
- `Switch`: wraps bits-ui `Switch.Root/Thumb`
- `Badge`: just a `<span>` with cn() variant classes (default=emerald, warning=amber)

---

### Task 5: Re-skin App.svelte

**Files:**

- Modify: `src/App.svelte`

Remove `<style>` block. Replace `.brand`, `.mark`, `.word`, `.drop-overlay` classes with Tailwind inline classes. Keep all script logic untouched.

Key mappings:

- `.brand` → `absolute top-[22px] left-[26px] z-30 inline-flex items-center gap-2 text-[13px] tracking-tight select-none`
- `.mark` → `text-[10px] text-[#4ade80]`
- `.word` → `text-foreground font-medium`
- `.drop-overlay` → `fixed inset-[9px] z-70 grid place-items-center bg-black/70 backdrop-blur-sm border border-dashed border-[#4ade80] rounded-sm animate-[pop_0.15s_ease]`

---

### Task 6: Re-skin EmptyState.svelte

**Files:**

- Modify: `src/lib/components/EmptyState.svelte`

Remove `<style>`. Replace with Tailwind. Keep layers SVG, headline, sub, Select Files button (use shadcn Button), paste hint, kbd spans, animations (use keyframe via CSS-in-JS style or @keyframes in app.css).

---

### Task 7: Re-skin Toolbar.svelte (shadcn components)

**Files:**

- Modify: `src/lib/components/Toolbar.svelte`

This is the main work. Replace:

- `<select>` → shadcn `<Select>` with `aria-label="Output format"` on the trigger
- `<input type="range">` → shadcn `<Slider>`
- `<label class="check"><input type="checkbox">` × 2 → shadcn `<Switch>`
- `<button class="download">` → shadcn `<Button>`
- `.savings span` → shadcn `<Badge>` with `data-testid="savings"`

Remove `<style>` block. Use Tailwind for the bar layout, groups, separators.

---

### Task 8: Re-skin Viewer.svelte

**Files:**

- Modify: `src/lib/components/Viewer.svelte`

Remove `<style>`. Map classes to Tailwind. Zoom buttons use ghost `<Button>` variant with active state.

---

### Task 9: Re-skin Compare.svelte (Tailwind only, keep all logic)

**Files:**

- Modify: `src/lib/components/Compare.svelte`

Replace `<style>` with Tailwind classes. MUST keep:

- `img[alt="Compressed"]` and `img[alt="Original"]` (e2e reads these)
- Pointer event handlers
- clip-path inline style
- checker background pattern (can use inline style or Tailwind arbitrary value)

---

### Task 10: ESLint config update

**Files:**

- Modify: `eslint.config.js`

Add `'src/lib/components/ui/**'` to the `ignores` array.

---

### Task 11: Update e2e smoke tests

**Files:**

- Modify: `e2e/smoke.spec.ts`

shadcn Select renders as button + listbox, not native `<select>`. Replace `selectOption()` with:

```ts
async function selectFormat(page: Page, fmt: string): Promise<void> {
  await page.getByLabel('Output format').click();
  await page.getByRole('option', { name: fmt.toUpperCase() }).click();
}
```

Add `data-testid="savings"` to the savings element in Toolbar.svelte and update:

```ts
await expect(page.locator('[data-testid="savings"]')).toBeVisible();
```

(Or keep `.savings` class on the element — simpler, no selector change needed.)

---

### Task 12: Verify all checks pass

- [ ] `npm run lint` — 0 errors
- [ ] `npm run check` — 0 svelte-check errors
- [ ] `npm run build` — clean
- [ ] `npm run e2e` — all 4 tests green

Fix any failures before committing.

---

### Task 13: Commit

```powershell
git -C C:\Users\Tomy\Documents\compressor add -A
git -C C:\Users\Tomy\Documents\compressor commit -m "feat(playground): pivot UI a shadcn-svelte (Tailwind v4 + bits-ui), tema dark + IBM Plex Mono"
```
