<script lang="ts">
  import type { Difficulty } from "../protocol/Difficulty";
  import type { MatchConfig } from "../protocol/MatchConfig";
  import type { SeatConfig } from "../protocol/SeatConfig";

  let {
    mode,
    onStart,
    onBack,
  }: {
    mode: "pass" | "bots";
    onStart: (config: MatchConfig) => void;
    onBack: () => void;
  } = $props();

  const DIFFICULTIES: Difficulty[] = ["Easy", "Medium", "Hard"];

  let names = $state(["Player 1", "Player 2"]);
  let yourName = $state("You");
  let bots = $state<Difficulty[]>(["Medium"]);
  let holes = $state(9);

  const humanCount = $derived(mode === "pass" ? names.length : 1);
  const total = $derived(mode === "pass" ? names.length : 1 + bots.length);

  function setHumanCount(n: number) {
    names = Array.from({ length: n }, (_, i) => names[i] ?? `Player ${i + 1}`);
  }

  function start() {
    const seats: SeatConfig[] =
      mode === "pass"
        ? names.map((name, i) => ({
            name: name.trim() || `Player ${i + 1}`,
            kind: { type: "Human" as const },
          }))
        : [
            { name: yourName.trim() || "You", kind: { type: "Human" as const } },
            ...bots.map((difficulty, i) => ({
              name: `${difficulty} bot ${bots.length > 1 ? i + 1 : ""}`.trim(),
              kind: { type: "Bot" as const, difficulty },
            })),
          ];
    onStart({ seats, holes });
  }
</script>

<div class="setup">
  <header>
    <button class="quiet small" onclick={onBack}>← Back</button>
    <h2>{mode === "pass" ? "Pass & play" : "Play the bots"}</h2>
    <span class="spacer"></span>
  </header>

  {#if mode === "pass"}
    <section>
      <h3>Players</h3>
      <div class="count">
        {#each [2, 3, 4] as n (n)}
          <button class:on={names.length === n} onclick={() => setHumanCount(n)}>{n}</button>
        {/each}
      </div>
      {#each names as _, i (i)}
        <input bind:value={names[i]} maxlength="14" aria-label={`Player ${i + 1} name`} />
      {/each}
    </section>
  {:else}
    <section>
      <h3>Your name</h3>
      <input bind:value={yourName} maxlength="14" aria-label="Your name" />
    </section>
    <section>
      <h3>Bots</h3>
      {#each bots as _, i (i)}
        <div class="bot-row">
          <span>🤖 Bot {i + 1}</span>
          <select bind:value={bots[i]} aria-label={`Bot ${i + 1} difficulty`}>
            {#each DIFFICULTIES as d (d)}
              <option value={d}>{d}</option>
            {/each}
          </select>
          {#if bots.length > 1}
            <button
              class="quiet small"
              onclick={() => (bots = bots.toSpliced(i, 1))}
              aria-label={`Remove bot ${i + 1}`}>✕</button
            >
          {/if}
        </div>
      {/each}
      {#if total < 4}
        <button class="secondary small" onclick={() => (bots = [...bots, "Medium"])}>
          + Add a bot
        </button>
      {/if}
    </section>
  {/if}

  <section>
    <h3>Holes</h3>
    <div class="count">
      {#each [1, 3, 9, 18] as n (n)}
        <button class:on={holes === n} onclick={() => (holes = n)}>{n}</button>
      {/each}
    </div>
  </section>

  <button class="primary tee" onclick={start} disabled={humanCount + (mode === "bots" ? bots.length : 0) < 2}>
    Tee off
  </button>
</div>

<style>
  .setup {
    min-height: 100dvh;
    max-width: 24rem;
    margin: 0 auto;
    padding: 1rem 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  header {
    display: flex;
    align-items: center;
  }
  header h2 {
    flex: 1;
    text-align: center;
    font-family: var(--display, serif);
    color: var(--cream, #f6f0dd);
    margin: 0;
    font-size: 1.4rem;
  }
  .spacer {
    width: 4rem;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  h3 {
    font-size: 0.75rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(246, 240, 221, 0.55);
    margin: 0;
    font-weight: 600;
  }

  .count {
    display: flex;
    gap: 0.5rem;
  }
  .count button {
    flex: 1;
    padding: 0.55rem 0;
    border-radius: 8px;
    border: 1px solid rgba(246, 240, 221, 0.3);
    background: transparent;
    color: var(--cream, #f6f0dd);
    font-weight: 600;
  }
  .count button.on {
    background: var(--sand, #d9b465);
    border-color: var(--sand, #d9b465);
    color: var(--ink, #26241f);
  }

  .bot-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--cream, #f6f0dd);
  }
  .bot-row span {
    flex: 1;
  }

  .tee {
    margin-top: auto;
  }
</style>
