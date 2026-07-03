<script lang="ts">
  import type { Card } from "../protocol/Card";
  import { RANK_SYMBOL, SUIT_SYMBOL, isRed } from "../cards";

  let {
    card = null,
    peeked = false,
    size = "md",
    interactive = false,
    highlight = false,
    pairColor = null,
    onclick,
  }: {
    /** null = face-down */
    card?: Card | null;
    /** face-down to others, but shown to the owner */
    peeked?: boolean;
    size?: "sm" | "md" | "lg";
    interactive?: boolean;
    highlight?: boolean;
    /** scoreboard pair highlight color */
    pairColor?: string | null;
    onclick?: () => void;
  } = $props();

  // Play a flip animation when a face-down card turns face-up.
  const showsFace = $derived(card !== null && !peeked);
  let wasDown: boolean | null = null; // unknown until the first effect run
  let flipping = $state(false);
  $effect(() => {
    const down = !showsFace;
    if (wasDown === true && !down) {
      flipping = true;
      const timer = setTimeout(() => (flipping = false), 450);
      wasDown = down;
      return () => clearTimeout(timer);
    }
    wasDown = down;
  });
</script>

{#if interactive}
  <button
    class="card {size}"
    class:face-down={card === null || peeked}
    class:peeked
    class:highlight
    class:flipping
    {onclick}
  >
    {#if card}
      <span class="face" class:red={isRed(card.suit)}>
        <span class="rank">{RANK_SYMBOL[card.rank]}</span>
        <span class="suit">{SUIT_SYMBOL[card.suit]}</span>
      </span>
    {/if}
    {#if peeked}<span class="eye" aria-label="Only you can see this">👁</span>{/if}
  </button>
{:else}
  <div
    class="card {size}"
    class:face-down={card === null || peeked}
    class:peeked
    class:highlight
    class:flipping
    style:box-shadow={pairColor ? `0 0 0 3px ${pairColor}` : undefined}
  >
    {#if card}
      <span class="face" class:red={isRed(card.suit)}>
        <span class="rank">{RANK_SYMBOL[card.rank]}</span>
        <span class="suit">{SUIT_SYMBOL[card.suit]}</span>
      </span>
    {/if}
    {#if peeked}<span class="eye" aria-label="Only you can see this">👁</span>{/if}
  </div>
{/if}

<style>
  .card {
    position: relative;
    aspect-ratio: 5 / 7;
    border-radius: 8%/ 5.7%;
    background: #fdfbf4;
    border: 1px solid rgba(30, 28, 22, 0.35);
    box-shadow: 0 1px 3px rgba(8, 20, 14, 0.45);
    display: grid;
    place-items: center;
    padding: 0;
    font: inherit;
    color: inherit;
    flex: none;
  }
  .card.sm {
    width: var(--card-sm, 34px);
  }
  .card.md {
    width: var(--card-md, 56px);
  }
  .card.lg {
    width: var(--card-lg, 72px);
  }

  /* Argyle back — the golf-sweater signature. */
  .face-down {
    background-color: var(--fairway-deep, #143526);
    background-image:
      repeating-linear-gradient(
        45deg,
        transparent 0,
        transparent 9px,
        rgba(246, 240, 221, 0.16) 9px,
        rgba(246, 240, 221, 0.16) 10px
      ),
      repeating-linear-gradient(
        -45deg,
        transparent 0,
        transparent 9px,
        rgba(246, 240, 221, 0.16) 9px,
        rgba(246, 240, 221, 0.16) 10px
      ),
      repeating-conic-gradient(
        from 45deg,
        #1c4732 0deg 90deg,
        #16382a 90deg 180deg
      );
    background-size:
      auto,
      auto,
      20px 20px;
    border-color: rgba(10, 26, 18, 0.8);
  }

  /* Peeked: argyle back with the value revealed only on this device/viewer. */
  .peeked .face {
    background: rgba(253, 251, 244, 0.92);
    border-radius: 6px;
    padding: 0.05em 0.28em;
  }
  .eye {
    position: absolute;
    top: -0.5em;
    right: -0.35em;
    font-size: 0.8em;
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.6));
  }

  .face {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1;
    color: var(--card-black, #23211c);
    font-weight: 650;
  }
  .face.red {
    color: var(--card-red, #b3352d);
  }
  .rank {
    font-size: clamp(0.85rem, 2.6vh, 1.3rem);
    font-variant-numeric: tabular-nums;
  }
  .suit {
    font-size: clamp(0.8rem, 2.2vh, 1.15rem);
  }
  .sm .rank {
    font-size: 0.8rem;
  }
  .sm .suit {
    font-size: 0.7rem;
  }

  button.card.face-down:not(:disabled) {
    cursor: pointer;
  }
  .highlight {
    outline: 3px solid var(--sand, #d9b465);
    outline-offset: 2px;
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      outline-color: rgba(217, 180, 101, 0.35);
    }
  }

  .flipping {
    animation: flip-in 400ms ease-out;
  }
  @keyframes flip-in {
    0% {
      transform: rotateY(88deg);
    }
    100% {
      transform: rotateY(0deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .highlight,
    .flipping {
      animation: none;
    }
  }
</style>
