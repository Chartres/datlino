<script lang="ts">
  import {
    FINGER_COLORS,
    FINGER_LABELS,
    keyForChar,
    US_LAYOUT,
    type KeyDef
  } from './keyboard-layout';

  type Props = {
    /** The character the student is supposed to type next. */
    nextChar: string | null;
    /** Whether to dim the keyboard (e.g. after session ends). */
    dim?: boolean;
  };
  let { nextChar, dim = false }: Props = $props();

  const target = $derived(nextChar ? keyForChar(nextChar) : null);

  function isKeyActive(key: KeyDef): boolean {
    if (!target) return false;
    if (key.base === ' ' && target.base === ' ') return true;
    if (key.base.toLowerCase() === target.base.toLowerCase()) return true;
    return false;
  }

  function isShiftActive(): boolean {
    return target?.shift ?? false;
  }
</script>

<div class="keyboard" class:dim aria-label="on-screen keyboard hint">
  {#if target?.dead}
    <div class="dead-hint">
      dead-key: nejdřív <kbd>{target.dead.via}</kbd> pak <kbd>{target.dead.then}</kbd>
    </div>
  {/if}
  {#each US_LAYOUT as row, i}
    <div class="row">
      {#each row as key (i + ':' + key.base)}
        {@const active = isKeyActive(key)}
        {@const shiftKey = key.base === 'Shift-L' || key.base === 'Shift-R'}
        {@const shiftGlow = shiftKey && isShiftActive()}
        <div
          class="key"
          class:active
          class:shift-glow={shiftGlow}
          style={`--finger:${FINGER_COLORS[key.finger]}; flex-grow:${key.width ?? 1};`}
          title={FINGER_LABELS[key.finger]}
        >
          {#if key.label}
            <span class="label">{key.label}</span>
          {:else}
            <span class="base">{key.base}</span>
            {#if key.shift && key.shift !== key.base.toUpperCase()}
              <span class="shift">{key.shift}</span>
            {/if}
          {/if}
          {#if key.homeDot}
            <span class="home-dot"></span>
          {/if}
        </div>
      {/each}
    </div>
  {/each}

  <div class="legend">
    {#each Object.entries(FINGER_LABELS) as [finger, label]}
      <div class="legend-item">
        <span class="swatch" style={`background:${FINGER_COLORS[finger as keyof typeof FINGER_COLORS]}`}></span>
        <span>{label}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .keyboard {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    user-select: none;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  .keyboard.dim {
    opacity: 0.4;
  }
  .row {
    display: flex;
    gap: 0.2rem;
  }
  .key {
    flex-basis: 0;
    flex-grow: 1;
    min-height: 2.4rem;
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    border-radius: 5px;
    background: #fff;
    border-top: 1px solid rgba(28, 25, 23, 0.08);
    border-bottom: 3px solid var(--finger);
    font-size: 0.85rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #1c1917;
    transition:
      transform 100ms,
      background 100ms,
      box-shadow 100ms;
  }
  .key.active {
    background: var(--finger);
    color: #fffaf2;
    transform: translateY(1px);
    box-shadow:
      0 0 0 2px #1c1917,
      0 0 18px 2px var(--finger);
    z-index: 1;
  }
  .key.shift-glow {
    background: color-mix(in srgb, var(--finger), white 70%);
    box-shadow: 0 0 0 2px var(--finger);
  }
  .key .label {
    font-size: 0.75rem;
    color: #78716c;
  }
  .key.active .label {
    color: #fffaf2;
  }
  .key .shift {
    position: absolute;
    top: 0.15rem;
    left: 0.25rem;
    font-size: 0.6rem;
    color: #a8a29e;
  }
  .key.active .shift {
    color: #fffaf2;
  }
  .key .base {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .home-dot {
    position: absolute;
    bottom: 0.25rem;
    width: 6px;
    height: 6px;
    background: rgba(28, 25, 23, 0.4);
    border-radius: 50%;
  }
  .key.active .home-dot {
    background: rgba(255, 250, 242, 0.9);
  }

  .dead-hint {
    font-size: 0.8rem;
    color: #b3271f;
    padding: 0.25rem 0.5rem;
    background: rgba(179, 39, 31, 0.08);
    border-radius: 4px;
    align-self: center;
  }
  .dead-hint kbd {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 3px;
    padding: 0.05rem 0.3rem;
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    margin: 0 0.1rem;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px dashed rgba(28, 25, 23, 0.08);
    font-size: 0.7rem;
    color: #57534e;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    display: inline-block;
  }
</style>
