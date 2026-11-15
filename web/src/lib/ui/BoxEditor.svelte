<script lang="ts">
  // Draw, move and resize boxes over an image preview (use inside ImageStage's overlay).
  // Boxes are fractions of the image. Drag on empty space to draw; drag a box to move it;
  // drag a corner to resize; arrow keys nudge the focused box; Delete removes it.
  import { moveBox, resizeBox, span, usable, type Box, type Corner } from '$lib/logic/box-edit';

  let {
    boxes = $bindable([]),
    width,
    height,
    max = 1,
    ratio,
    tone = 'blue',
    label = 'Selection',
  }: {
    boxes?: Box[];
    width: number;
    height: number;
    /** How many boxes may exist; drawing a new one past the limit replaces the last. */
    max?: number;
    /** Fixed width / height in fraction units. */
    ratio?: number;
    tone?: 'blue' | 'dark';
    label?: string;
  } = $props();

  type Drag =
    | { kind: 'draw'; ax: number; ay: number; index: number }
    | { kind: 'move'; index: number; px: number; py: number; start: Box }
    | { kind: 'resize'; index: number; corner: Corner; start: Box };

  let drag: Drag | null = null;
  let surface: HTMLDivElement;

  function point(e: PointerEvent) {
    const r = surface.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height };
  }

  function set(i: number, b: Box) {
    boxes = boxes.map((old, j) => (j === i ? b : old));
  }

  function onSurfaceDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const p = point(e);
    const b = { x: p.x, y: p.y, w: 0, h: 0 };
    let index: number;
    if (boxes.length < max) {
      boxes = [...boxes, b];
      index = boxes.length - 1;
    } else {
      index = boxes.length - 1;
      set(index, b);
    }
    drag = { kind: 'draw', ax: p.x, ay: p.y, index };
    surface.setPointerCapture(e.pointerId);
  }

  function onBoxDown(e: PointerEvent, index: number) {
    if (e.button !== 0) return;
    e.stopPropagation();
    const p = point(e);
    drag = { kind: 'move', index, px: p.x, py: p.y, start: boxes[index] };
    surface.setPointerCapture(e.pointerId);
  }

  function onGripDown(e: PointerEvent, index: number, corner: Corner) {
    if (e.button !== 0) return;
    e.stopPropagation();
    drag = { kind: 'resize', index, corner, start: boxes[index] };
    surface.setPointerCapture(e.pointerId);
  }

  function onMove(e: PointerEvent) {
    if (!drag) return;
    const p = point(e);
    if (drag.kind === 'draw') set(drag.index, span(drag.ax, drag.ay, p.x, p.y, ratio));
    else if (drag.kind === 'move') set(drag.index, moveBox(drag.start, p.x - drag.px, p.y - drag.py));
    else set(drag.index, resizeBox(drag.start, drag.corner, p.x, p.y, ratio));
  }

  function onUp() {
    if (drag?.kind === 'draw' && !usable(boxes[drag.index])) {
      // A click rather than a drag: drop the empty box.
      const i = drag.index;
      boxes = boxes.filter((_, j) => j !== i);
    }
    drag = null;
  }

  function onKey(e: KeyboardEvent, index: number) {
    const step = e.shiftKey ? 0.05 : 0.01;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (moves[e.key]) {
      e.preventDefault();
      set(index, moveBox(boxes[index], ...moves[e.key]));
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && max > 1) {
      e.preventDefault();
      boxes = boxes.filter((_, j) => j !== index);
    }
  }

  const corners: Corner[] = ['nw', 'ne', 'sw', 'se'];
</script>

<div
  class="surface {tone}"
  bind:this={surface}
  style="width: {width}px; height: {height}px"
  onpointerdown={onSurfaceDown}
  onpointermove={onMove}
  onpointerup={onUp}
  onpointercancel={onUp}
  role="presentation"
>
  {#each boxes as b, i (i)}
    <div
      class="box"
      style="left: {b.x * 100}%; top: {b.y * 100}%; width: {b.w * 100}%; height: {b.h * 100}%"
      onpointerdown={(e) => onBoxDown(e, i)}
      onkeydown={(e) => onKey(e, i)}
      tabindex="0"
      role="button"
      aria-label="{label} {max > 1 ? i + 1 : ''}: drag to move, arrow keys to nudge{max > 1 ? ', Delete to remove' : ''}"
    >
      {#each corners as c (c)}
        <span class="grip {c}" aria-hidden="true" onpointerdown={(e) => onGripDown(e, i, c)}></span>
      {/each}
      {#if max > 1}
        <button
          type="button"
          class="remove"
          aria-label="Remove {label.toLowerCase()} {i + 1}"
          onpointerdown={(e) => e.stopPropagation()}
          onclick={() => (boxes = boxes.filter((_, j) => j !== i))}>×</button
        >
      {/if}
    </div>
  {/each}
</div>

<style>
  .surface {
    position: relative;
    cursor: crosshair;
    touch-action: none;
  }

  .box {
    position: absolute;
    cursor: move;
    outline: 2px solid var(--straw);
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.6);
  }

  .blue .box {
    background: rgb(47 75 216 / 0.12);
  }

  .dark .box {
    background: rgb(0 0 0 / 0.35);
  }

  .box:focus-visible {
    outline: 3px solid var(--blue);
  }

  .grip {
    position: absolute;
    width: 14px;
    height: 14px;
    background: var(--straw);
    border: 1px solid rgb(0 0 0 / 0.6);
  }

  .nw {
    left: -7px;
    top: -7px;
    cursor: nwse-resize;
  }
  .ne {
    right: -7px;
    top: -7px;
    cursor: nesw-resize;
  }
  .sw {
    left: -7px;
    bottom: -7px;
    cursor: nesw-resize;
  }
  .se {
    right: -7px;
    bottom: -7px;
    cursor: nwse-resize;
  }

  .remove {
    position: absolute;
    right: 4px;
    top: 4px;
    padding: 0 6px;
    line-height: 1.4;
    font-size: var(--step-0);
  }
</style>
