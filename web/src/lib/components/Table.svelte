<script lang="ts">
  import type { Emote } from "../protocol/Emote";
  import type { GameSession } from "../session/types";
  import { cardLabel } from "../cards";
  import { ALL_EMOTES, EMOTE_GLYPH } from "../emotes";
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

  let emotesOpen = $state(false);
  const emotes = $derived(session.emotes ?? {});
  /** Room roster; mid-game a kicked seat flips to a bot here. */
  const roster = $derived(session.lobby?.seats ?? null);

  function isKickable(s: number): boolean {
    return (
      session.isHost === true &&
      session.removeSeat !== undefined &&
      roster !== null &&
      roster[s] != null &&
      !roster[s].difficulty
    );
  }

  function kick(s: number) {
    const name = view?.seats[s]?.name ?? "this player";
    if (confirm(`Remove ${name} from the table? A bot takes over their seat.`)) {
      session.removeSeat?.(s);
    }
  }

  function sendEmote(e: Emote) {
    session.sendEmote?.(e);
    emotesOpen = false;
  }

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
      {#if session.sendEmote}
        <button
          class="quiet small emote-toggle"
          class:open={emotesOpen}
          onclick={() => (emotesOpen = !emotesOpen)}
          aria-label="Send a reaction"
          aria-expanded={emotesOpen}>😊</button
        >
      {:else}
        <span class="spacer"></span>
      {/if}
    </header>

    {#if emotesOpen}
      <div class="emote-palette" role="group" aria-label="Reactions">
        {#each ALL_EMOTES as e (e)}
          <button onclick={() => sendEmote(e)} aria-label={e}>{EMOTE_GLYPH[e]}</button>
        {/each}
      </div>
    {/if}

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
              {seat.name}{seat.is_bot || roster?.[s]?.difficulty ? " 🤖" : ""} · {view.totals[s] >
              0
                ? "+"
                : ""}{view.totals[s]}
            </span>
            {#if emotes[s]}
              {#key emotes[s].n}
                <span class="bubble">{EMOTE_GLYPH[emotes[s].emote]}</span>
              {/key}
            {/if}
            {#if isKickable(s)}
              <button class="kick" onclick={() => kick(s)} aria-label={`Remove ${seat.name}`}
                >✕</button
              >
            {/if}
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
        {#if emotes[me]}
          {#key emotes[me].n}
            <span class="bubble">{EMOTE_GLYPH[emotes[me].emote]}</span>
          {/key}
        {/if}
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
    position: relative;
  }

  .emote-toggle {
    width: 4.5rem;
    text-align: right;
    font-size: 1.05rem;
  }
  .emote-toggle.open {
    opacity: 0.6;
  }
  .emote-palette {
    position: absolute;
    top: 2.4rem;
    right: 0.75rem;
    z-index: 5;
    display: flex;
    gap: 0.3rem;
    padding: 0.35rem;
    border-radius: 12px;
    background: rgba(38, 36, 31, 0.95);
    border: 1px solid rgba(246, 240, 221, 0.2);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
  }
  .emote-palette button {
    font-size: 1.3rem;
    line-height: 1;
    padding: 0.35rem 0.4rem;
    border-radius: 8px;
    background: transparent;
    border: none;
  }
  .emote-palette button:active {
    background: rgba(217, 180, 101, 0.3);
  }

  .bubble {
    position: absolute;
    top: -0.6rem;
    right: -0.4rem;
    font-size: 1.5rem;
    line-height: 1;
    padding: 0.15rem;
    border-radius: 50%;
    background: rgba(38, 36, 31, 0.85);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
    animation: bubble-pop 0.35s cubic-bezier(0.2, 1.6, 0.4, 1);
    pointer-events: none;
    z-index: 4;
  }
  @keyframes bubble-pop {
    from {
      transform: scale(0.2) translateY(0.4rem);
      opacity: 0;
    }
    to {
      transform: scale(1) translateY(0);
      opacity: 1;
    }
  }

  .kick {
    position: absolute;
    top: -0.35rem;
    left: -0.35rem;
    font-size: 0.7rem;
    line-height: 1;
    width: 1.3rem;
    height: 1.3rem;
    border-radius: 50%;
    border: 1px solid rgba(200, 68, 44, 0.6);
    background: rgba(38, 36, 31, 0.9);
    color: var(--flag, #c8442c);
    z-index: 4;
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
    position: relative;
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
    position: relative;
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
