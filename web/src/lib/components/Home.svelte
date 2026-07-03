<script lang="ts">
  let {
    onPassPlay,
    onBots,
    onCreateRoom,
    onJoinRoom,
  }: {
    onPassPlay: () => void;
    onBots: () => void;
    onCreateRoom: () => void;
    onJoinRoom: (code: string) => void;
  } = $props();

  let code = $state("");
</script>

<div class="home">
  <div class="hero">
    <p class="eyebrow">— The card game —</p>
    <h1>Golf</h1>
    <p class="tagline">Six cards, six turns, nine holes.<br />Lowest score wins the round.</p>
  </div>

  <div class="menu">
    <section>
      <h2>On this device</h2>
      <button class="primary" onclick={onPassPlay}>Pass &amp; play</button>
      <button class="primary" onclick={onBots}>Play the bots</button>
    </section>

    <section>
      <h2>Online</h2>
      <button class="secondary" onclick={onCreateRoom}>Open a table</button>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          if (code.trim()) onJoinRoom(code.trim().toUpperCase());
        }}
      >
        <input
          bind:value={code}
          placeholder="Table code"
          maxlength="4"
          autocapitalize="characters"
          autocomplete="off"
          spellcheck="false"
          aria-label="Table code"
        />
        <button class="secondary" type="submit" disabled={code.trim().length < 4}>Join</button>
      </form>
    </section>
  </div>

  <p class="rules-hint">
    Pairs score zero. Aces pair with anything for −2. Face cards cost 10.
  </p>
</div>

<style>
  .home {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2rem;
    padding: 2rem 1.5rem;
    text-align: center;
  }
  .hero h1 {
    font-family: var(--display, serif);
    font-size: clamp(4rem, 18vw, 6.5rem);
    font-weight: 640;
    line-height: 0.9;
    margin: 0.1em 0;
    color: var(--cream, #f6f0dd);
    letter-spacing: 0.01em;
  }
  .tagline {
    color: rgba(246, 240, 221, 0.75);
    line-height: 1.5;
  }

  .menu {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    width: min(20rem, 100%);
  }
  .menu section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .menu h2 {
    font-size: 0.75rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(246, 240, 221, 0.55);
    margin: 0;
    font-weight: 600;
  }
  form {
    display: flex;
    gap: 0.5rem;
  }
  input {
    flex: 1;
    min-width: 0;
    text-transform: uppercase;
    text-align: center;
    letter-spacing: 0.35em;
    font-weight: 700;
  }
  form button {
    flex: none;
  }

  .rules-hint {
    font-size: 0.8rem;
    color: rgba(246, 240, 221, 0.5);
    max-width: 34ch;
  }
</style>
