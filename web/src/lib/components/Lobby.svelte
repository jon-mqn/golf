<script lang="ts">
  import type { Difficulty } from "../protocol/Difficulty";
  import type { OnlineSession } from "../session/online.svelte";

  let {
    session,
    onLeave,
  }: {
    session: OnlineSession;
    onLeave: () => void;
  } = $props();

  const DIFFICULTIES: Difficulty[] = ["Easy", "Medium", "Hard"];
  let difficulty = $state<Difficulty>("Medium");
  let holes = $state(9);
  let copied = $state(false);

  const lobby = $derived(session.lobby);
  const shareUrl = $derived(session.code ? `${location.origin}/room/${session.code}` : "");

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(shareUrl);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      prompt("Copy this link:", shareUrl);
    }
  }
</script>

<div class="lobby">
  <header>
    <button class="quiet small" onclick={onLeave}>← Leave</button>
    <h2>Table {session.code ?? "…"}</h2>
    <span class="spacer"></span>
  </header>

  {#if session.status === "connecting"}
    <p class="status">Connecting…</p>
  {:else if session.status === "reconnecting"}
    <p class="status">Connection lost — reconnecting…</p>
  {:else if lobby}
    <section class="share">
      <p class="code" aria-label="table code">{session.code}</p>
      <button class="secondary" onclick={copyLink}>
        {copied ? "Copied ✓" : "Copy invite link"}
      </button>
      <p class="hint">Friends join with the code or the link.</p>
    </section>

    <section>
      <h3>Seats · {lobby.seats.length}/4</h3>
      <ul>
        {#each lobby.seats as seat, i (i)}
          <li>
            <span class="name">
              {seat.difficulty ? "🤖" : "🧑"}
              {seat.name}
              {#if i === lobby.host}<span class="badge">host</span>{/if}
              {#if i === lobby.you}<span class="badge you">you</span>{/if}
            </span>
            {#if !seat.connected && !seat.difficulty}
              <span class="offline">offline</span>
            {/if}
            {#if session.isHost && seat.difficulty}
              <button
                class="quiet small"
                onclick={() => session.removeSeat(i)}
                aria-label={`Remove ${seat.name}`}>✕</button
              >
            {/if}
          </li>
        {/each}
      </ul>
      {#if session.isHost && lobby.seats.length < 4}
        <div class="add-bot">
          <select bind:value={difficulty} aria-label="Bot difficulty">
            {#each DIFFICULTIES as d (d)}
              <option value={d}>{d}</option>
            {/each}
          </select>
          <button class="secondary" onclick={() => session.addBot(difficulty)}>+ Add a bot</button>
        </div>
      {/if}
    </section>

    {#if session.isHost}
      <section>
        <h3>Holes</h3>
        <div class="count">
          {#each [1, 3, 9, 18] as n (n)}
            <button class:on={holes === n} onclick={() => (holes = n)}>{n}</button>
          {/each}
        </div>
      </section>
      <button
        class="primary tee"
        disabled={lobby.seats.length < 2}
        onclick={() => session.startMatch(holes)}
      >
        {lobby.seats.length < 2 ? "Waiting for players…" : "Tee off"}
      </button>
    {:else}
      <p class="status">Waiting for {lobby.seats[lobby.host]?.name ?? "the host"} to tee off…</p>
    {/if}
  {/if}

  {#if session.error}
    <p class="error">{session.error}</p>
  {/if}
</div>

<style>
  .lobby {
    min-height: 100dvh;
    max-width: 24rem;
    margin: 0 auto;
    padding: 1rem 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
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

  .share {
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: center;
  }
  .code {
    font-family: var(--display, serif);
    font-size: 3rem;
    letter-spacing: 0.25em;
    margin: 0;
    color: var(--sand, #d9b465);
  }
  .hint {
    font-size: 0.8rem;
    color: rgba(246, 240, 221, 0.6);
    margin: 0;
  }

  h3 {
    font-size: 0.75rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(246, 240, 221, 0.55);
    margin: 0 0 0.5rem;
    font-weight: 600;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(246, 240, 221, 0.07);
    border-radius: 10px;
    padding: 0.55rem 0.8rem;
  }
  .name {
    flex: 1;
  }
  .badge {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    background: rgba(217, 180, 101, 0.25);
    border-radius: 6px;
    padding: 0.1em 0.5em;
    margin-left: 0.4em;
  }
  .badge.you {
    background: rgba(158, 199, 184, 0.25);
  }
  .offline {
    font-size: 0.7rem;
    color: var(--flag, #c8442c);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .add-bot {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.6rem;
  }
  .add-bot select {
    flex: 1;
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

  .tee {
    margin-top: auto;
  }
  .status {
    text-align: center;
    color: rgba(246, 240, 221, 0.7);
  }
  .error {
    text-align: center;
    color: var(--flag, #c8442c);
    font-weight: 600;
  }
</style>
