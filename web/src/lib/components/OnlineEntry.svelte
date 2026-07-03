<script lang="ts">
  let {
    code,
    onSubmit,
    onBack,
  }: {
    /** null = opening a new table; otherwise the code being joined */
    code: string | null;
    onSubmit: (name: string) => void;
    onBack: () => void;
  } = $props();

  let name = $state("");
</script>

<div class="entry">
  <header>
    <button class="quiet small" onclick={onBack}>← Back</button>
    <h2>{code ? `Join table ${code}` : "Open a table"}</h2>
    <span class="spacer"></span>
  </header>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      onSubmit(name.trim() || "Player");
    }}
  >
    <label for="player-name">Your name</label>
    <input id="player-name" bind:value={name} maxlength="14" placeholder="Player" />
    <button class="primary" type="submit">
      {code ? "Take a seat" : "Open the table"}
    </button>
  </form>
</div>

<style>
  .entry {
    min-height: 100dvh;
    max-width: 24rem;
    margin: 0 auto;
    padding: 1rem 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
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
  form {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  label {
    font-size: 0.75rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(246, 240, 221, 0.55);
    font-weight: 600;
  }
</style>
