<script lang="ts">
  import type { PlayerView } from "../protocol/PlayerView";
  import PlayerGrid from "./PlayerGrid.svelte";

  let {
    view,
    onNextHole,
    onExit,
    onRematch,
  }: {
    view: PlayerView;
    onNextHole: () => void;
    onExit: () => void;
    onRematch?: () => void;
  } = $props();

  const phase = $derived(view.phase);
  const isMatchOver = $derived(phase.type === "MatchComplete");
  const winners = $derived(phase.type === "MatchComplete" ? phase.winners : []);
  const holes = $derived(Array.from({ length: view.holes_total }, (_, i) => i + 1));
  const canAct = $derived(view.legal_actions.includes("NextHole"));

  function holeScore(seat: number, hole: number): string {
    const h = view.score_history[hole - 1];
    return h === undefined ? "" : String(h[seat]);
  }
</script>

<div class="backdrop" role="dialog" aria-modal="true">
  <div class="scorecard">
    <header>
      <p class="eyebrow">— Scorecard —</p>
      {#if isMatchOver}
        <h2>
          {winners.length > 1 ? "Shared round" : `${view.seats[winners[0]].name} wins the round`}
        </h2>
      {:else}
        <h2>Hole {view.hole_number} of {view.holes_total}</h2>
      {/if}
    </header>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th class="player">Player</th>
            {#each holes as h (h)}
              <th class:current={h === view.hole_number && !isMatchOver}>{h}</th>
            {/each}
            <th class="total">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each view.seats as seat, s (s)}
            <tr>
              <td class="player">
                {seat.name}{#if isMatchOver && winners.includes(s)}
                  <span class="flag" aria-label="winner">⛳</span>{/if}
              </td>
              {#each holes as h (h)}
                <td>{holeScore(s, h)}</td>
              {/each}
              <td class="total">{view.totals[s]}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="hands">
      {#each view.seats as seat, s (s)}
        <figure>
          <PlayerGrid
            grid={seat.grid}
            size="sm"
            pairs={phase.type === "HoleComplete" ? phase.pairings[s] : []}
          />
          <figcaption>
            {seat.name}
            {#if phase.type === "HoleComplete"}
              <strong>{phase.scores[s] > 0 ? "+" : ""}{phase.scores[s]}</strong>
            {/if}
          </figcaption>
        </figure>
      {/each}
    </div>
    {#if phase.type === "HoleComplete"}
      <p class="note">Matching pairs share a color. Aces pair with anything for −2.</p>
    {/if}

    <div class="actions">
      {#if isMatchOver}
        {#if onRematch}<button class="primary" onclick={onRematch}>Play again</button>{/if}
        <button class="quiet" onclick={onExit}>Leave table</button>
      {:else}
        <button class="primary" onclick={onNextHole} disabled={!canAct}>
          {!canAct
            ? "Waiting for the host…"
            : view.hole_number >= view.holes_total
              ? "See final results"
              : `Tee off hole ${view.hole_number + 1}`}
        </button>
        <button class="quiet" onclick={onExit}>Leave table</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: rgba(10, 26, 18, 0.75);
    display: flex;
    padding: 1rem;
    overflow-y: auto;
  }
  /* margin:auto centers when it fits but keeps the top reachable when the
     scorecard is taller than a phone viewport (grid place-items clips it). */
  .scorecard {
    margin: auto;
  }
  .scorecard {
    background: var(--cream, #f6f0dd);
    color: var(--ink, #26241f);
    border-radius: 10px;
    padding: 1.25rem 1.25rem 1rem;
    width: min(560px, 100%);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
    border: 1px solid #cdbf9d;
  }
  header {
    text-align: center;
    margin-bottom: 0.75rem;
  }
  header h2 {
    font-family: var(--display, serif);
    margin: 0.1rem 0 0;
    font-size: 1.5rem;
  }
  .eyebrow {
    color: #8a7c58;
  }

  .table-wrap {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
  }
  th,
  td {
    border: 1px solid #cdbf9d;
    padding: 0.3rem 0.4rem;
    text-align: center;
    min-width: 1.9em;
  }
  th {
    font-weight: 600;
    background: #ece3ca;
  }
  th.current {
    background: var(--sand, #d9b465);
  }
  .player {
    text-align: left;
    max-width: 9em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .total {
    font-weight: 700;
    background: #ece3ca;
  }
  .flag {
    margin-left: 0.25em;
  }

  .hands {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: center;
    margin-top: 1rem;
  }
  figure {
    margin: 0;
    text-align: center;
  }
  figcaption {
    font-size: 0.8rem;
    margin-top: 0.3rem;
    color: #57503c;
  }
  figcaption strong {
    margin-left: 0.3em;
  }
  .note {
    text-align: center;
    font-size: 0.75rem;
    color: #8a7c58;
    margin: 0.5rem 0 0;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
    align-items: center;
  }
</style>
