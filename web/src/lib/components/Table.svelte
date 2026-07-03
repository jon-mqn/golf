<script lang="ts">
  import type { GameSession } from "../session/types";
  import { cardLabel } from "../cards";
  import CardView from "./CardView.svelte";
  import PlayerGrid from "./PlayerGrid.svelte";
  import PassScreen from "./PassScreen.svelte";
  import Scoreboard from "./Scoreboard.svelte";

  let {
    session,
    onExit,
    onRematch,
  }: {
    session: GameSession;
    onExit: () => void;
    onRematch?: () => void;
  } = $props();

  const view = $derived(session.view);
  const me = $derived(view?.viewer ?? null);
  const legal = $derived(view?.legal_actions ?? []);
  const turn = $derived(view?.turn);
  const playing = $derived(view?.phase.type === "Playing");

  const currentName = $derived(view ? view.seats[view.current].name : "");
  const myTurn = $derived(view !== null && me !== null && view.current === me && playing);

  const drawnCard = $derived(
    turn && turn.type !== "AwaitDraw" && playing ? turn.drawn : null,
  );
  const flippedCard = $derived.by(() => {
    if (!view || !turn || turn.type !== "AwaitResolve") return null;
    const slot = view.seats[view.current].grid[turn.flipped];
    return slot.type === "FaceUp" ? slot.card : null;
  });

  const hint = $derived.by(() => {
    if (!view || !playing) return "";
    if (!myTurn) return `${currentName} is playing…`;
    switch (turn?.type) {
      case "AwaitDraw":
        return "Draw from the deck, or take the discard";
      case "AwaitFlip":
        return `You picked up ${cardLabel(turn.drawn)} — now flip one of your cards`;
      case "AwaitResolve":
        return "Swap in your pickup, or keep the card you flipped";
      default:
        return "";
    }
  });
</script>

{#if view}
  <div class="table">
    {#if session.status === "reconnecting"}
      <div class="banner">Connection lost — reconnecting…</div>
    {:else if session.error}
      <div class="banner error">{session.error}</div>
    {/if}
    <header>
      <button class="quiet small" onclick={onExit}>← Leave</button>
      <span class="hole">
        <span class="flag">⛳</span> Hole {view.hole_number} <em>of {view.holes_total}</em>
      </span>
      <span class="spacer"></span>
    </header>

    <section
      class="opponents"
      class:solo={view.seats.length === 2 && me !== null}
      class:trio={view.seats.length === 4 && me !== null}
    >
      {#each view.seats as seat, s (s)}
        {#if s !== me}
          <div class="opponent" class:active={playing && view.current === s}>
            <PlayerGrid grid={seat.grid} size="sm" />
            <span class="tag">
              {seat.name}{seat.is_bot ? " 🤖" : ""} · {view.totals[s] > 0 ? "+" : ""}{view.totals[s]}
            </span>
          </div>
        {/if}
      {/each}
    </section>

    <section class="center">
      <div class="pile">
        <CardView
          card={null}
          size="lg"
          interactive={legal.includes("DrawFromDeck")}
          highlight={legal.includes("DrawFromDeck")}
          onclick={() => session.send({ type: "DrawFromDeck" })}
        />
        <span class="tag">Deck · {view.deck_len}</span>
      </div>

      <div class="tray" class:empty={!drawnCard}>
        {#if drawnCard}
          <CardView card={drawnCard} size="lg" />
          <span class="tag">{myTurn ? "Your pickup" : `${currentName}'s pickup`}</span>
        {/if}
      </div>

      <div class="pile">
        {#if view.discard_top}
          <CardView
            card={view.discard_top}
            size="lg"
            interactive={legal.includes("TakeDiscard")}
            highlight={legal.includes("TakeDiscard")}
            onclick={() => session.send({ type: "TakeDiscard" })}
          />
        {:else}
          <div class="empty-pile"></div>
        {/if}
        <span class="tag">Discard</span>
      </div>
    </section>

    {#if me !== null}
      <section class="mine" class:active={myTurn}>
        <PlayerGrid
          grid={view.seats[me].grid}
          size="lg"
          selectable={legal.includes("Flip")}
          onFlip={(slot) => session.send({ type: "Flip", slot })}
        />
        <span class="tag self">
          {view.seats[me].name} · {view.totals[me] > 0 ? "+" : ""}{view.totals[me]}
        </span>
      </section>
    {/if}

    <footer>
      {#if legal.includes("Swap") && drawnCard && flippedCard}
        <div class="resolve">
          <button class="primary" onclick={() => session.send({ type: "Swap" })}>
            Swap in {cardLabel(drawnCard)}
          </button>
          <button class="secondary" onclick={() => session.send({ type: "Keep" })}>
            Keep {cardLabel(flippedCard)}
          </button>
        </div>
      {:else}
        <p class="hint" class:mine-turn={myTurn}>{hint}</p>
      {/if}
    </footer>
  </div>

  {#if view.phase.type !== "Playing"}
    <Scoreboard
      {view}
      onNextHole={() => session.send({ type: "NextHole" })}
      {onExit}
      {onRematch}
    />
  {/if}
{/if}

{#if session.passTo}
  <PassScreen name={session.passTo} onConfirm={() => session.confirmPass()} />
{/if}

<style>
  .banner {
    text-align: center;
    background: rgba(217, 180, 101, 0.18);
    border: 1px solid rgba(217, 180, 101, 0.5);
    color: var(--sand, #d9b465);
    border-radius: 8px;
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
  }
  .banner.error {
    color: var(--cream, #f6f0dd);
    background: rgba(200, 68, 44, 0.25);
    border-color: rgba(200, 68, 44, 0.6);
  }

  .table {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    padding: 0.4rem 0.75rem calc(0.5rem + env(safe-area-inset-bottom));
    max-width: 560px;
    margin: 0 auto;
    gap: 0.2rem;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .hole {
    flex: 1;
    text-align: center;
    font-family: var(--display, serif);
    font-size: 1.05rem;
    color: var(--cream, #f6f0dd);
  }
  .hole em {
    font-style: normal;
    opacity: 0.6;
    font-size: 0.85em;
  }
  .spacer {
    width: 4.5rem;
  }

  .opponents {
    display: flex;
    justify-content: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding: 0.2rem 0;
  }
  .opponent {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    border-radius: 10px;
    padding: 0.3rem 0.55rem;
    border: 1px solid transparent;
  }
  .opponent.active {
    border-color: rgba(217, 180, 101, 0.7);
    background: rgba(217, 180, 101, 0.08);
  }
  .opponents.solo .opponent {
    --card-sm: min(44px, 5.6dvh);
  }
  /* Three opponents must share one row on a phone. */
  .opponents.trio {
    gap: 0.4rem;
  }
  .opponents.trio .opponent {
    --card-sm: min(29px, 4.2dvh);
    padding: 0.3rem 0.3rem;
  }
  .opponents.trio .tag {
    font-size: 0.62rem;
  }

  .center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 0.25rem 0;
  }
  .pile,
  .tray {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
  }
  .tray {
    min-width: var(--card-lg, 72px);
    min-height: calc(var(--card-lg, 72px) * 1.4);
    justify-content: center;
  }
  .empty-pile {
    width: var(--card-lg, 72px);
    aspect-ratio: 5 / 7;
    border: 1px dashed rgba(246, 240, 221, 0.35);
    border-radius: 6px;
  }

  .mine {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    padding: 0.4rem 0.5rem;
    border-radius: 14px;
    border: 1px solid transparent;
  }
  .mine.active {
    border-color: rgba(217, 180, 101, 0.5);
    background: rgba(217, 180, 101, 0.06);
  }

  .tag {
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(246, 240, 221, 0.75);
  }
  .tag.self {
    color: var(--cream, #f6f0dd);
  }

  footer {
    min-height: 2.4rem;
    display: grid;
    place-items: center;
  }
  .hint {
    color: rgba(246, 240, 221, 0.8);
    text-align: center;
    margin: 0;
  }
  .hint.mine-turn {
    color: var(--sand, #d9b465);
    font-weight: 600;
  }
  .resolve {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    justify-content: center;
  }
  .resolve button {
    padding: 0.55rem 1.1rem;
  }
</style>
