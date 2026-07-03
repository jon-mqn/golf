<script lang="ts">
  let {
    name,
    onConfirm,
  }: {
    name: string;
    onConfirm: () => void;
  } = $props();
</script>

<div class="pass" role="dialog" aria-modal="true">
  <p class="eyebrow">Pass the device</p>
  <h2>{name}, you're up</h2>
  <p class="hint">Your cards stay hidden until you're holding the device.</p>
  <button class="primary" onclick={onConfirm}>I'm {name} — show my cards</button>
</div>

<style>
  .pass {
    position: fixed;
    inset: 0;
    z-index: 40;
    background:
      radial-gradient(120% 90% at 50% 0%, #1d4a33 0%, #143526 70%, #0e2a1e 100%);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    text-align: center;
    padding: 2rem;
  }
  h2 {
    font-family: var(--display, serif);
    font-size: 2rem;
    color: var(--cream, #f6f0dd);
    margin: 0;
  }
  .hint {
    color: rgba(246, 240, 221, 0.7);
    max-width: 26ch;
  }
</style>
