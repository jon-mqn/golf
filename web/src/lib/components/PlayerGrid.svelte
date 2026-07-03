<script lang="ts">
  import type { SlotView } from "../protocol/SlotView";
  import CardView from "./CardView.svelte";

  const PAIR_COLORS = ["#d9b465", "#9ec7b8", "#d98a65", "#b8a6d9"];

  let {
    grid,
    size = "md",
    selectable = false,
    pairs = [],
    onFlip,
  }: {
    grid: SlotView[];
    size?: "sm" | "md" | "lg";
    /** face-down cards may be tapped to flip */
    selectable?: boolean;
    /** slot-index pairs to highlight (scoreboard) */
    pairs?: [number, number][];
    onFlip?: (slot: number) => void;
  } = $props();

  function pairColor(slot: number): string | null {
    const i = pairs.findIndex(([a, b]) => a === slot || b === slot);
    return i >= 0 ? PAIR_COLORS[i % PAIR_COLORS.length] : null;
  }
</script>

<div class="grid {size}">
  {#each grid as slot, i (i)}
    {@const faceDown = slot.type !== "FaceUp"}
    <CardView
      card={slot.type === "Hidden" ? null : slot.card}
      peeked={slot.type === "Peeked"}
      {size}
      interactive={selectable && faceDown}
      highlight={selectable && faceDown}
      pairColor={pairColor(i)}
      onclick={() => onFlip?.(i)}
    />
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(3, auto);
    justify-content: center;
  }
  .grid.sm {
    gap: 3px;
  }
  .grid.md {
    gap: 6px;
  }
  .grid.lg {
    gap: 8px;
  }
</style>
