<script lang="ts">
  import type { MatchConfig } from "./lib/protocol/MatchConfig";
  import type { GameSession } from "./lib/session/types";
  import { createLocalSession } from "./lib/session/local.svelte";
  import { OnlineSession, savedToken } from "./lib/session/online.svelte";
  import Home from "./lib/components/Home.svelte";
  import LocalSetup from "./lib/components/LocalSetup.svelte";
  import Lobby from "./lib/components/Lobby.svelte";
  import OnlineEntry from "./lib/components/OnlineEntry.svelte";
  import Table from "./lib/components/Table.svelte";

  type Screen =
    | { t: "home" }
    | { t: "setup"; mode: "pass" | "bots" }
    | { t: "game"; session: GameSession; local: boolean }
    | { t: "online-entry"; code: string | null }
    | { t: "online"; session: OnlineSession };

  let screen = $state<Screen>(initialScreen());
  let lastConfig: MatchConfig | null = null;

  /** Deep links: /room/CODE joins (or rejoins with a saved token). */
  function initialScreen(): Screen {
    const match = location.pathname.match(/^\/room\/([A-Za-z0-9]{4})\/?$/);
    if (!match) return { t: "home" };
    const code = match[1].toUpperCase();
    const token = savedToken(code);
    if (token) {
      return { t: "online", session: new OnlineSession({ kind: "rejoin", code, token }) };
    }
    return { t: "online-entry", code };
  }

  function goHome() {
    history.replaceState(null, "", "/");
    screen = { t: "home" };
  }

  async function startLocal(config: MatchConfig) {
    lastConfig = config;
    const session = await createLocalSession(config);
    screen = { t: "game", session, local: true };
  }

  function exitGame() {
    if (screen.t === "game") screen.session.destroy();
    goHome();
  }

  async function rematch() {
    if (screen.t !== "game" || !lastConfig) return;
    screen.session.destroy();
    const session = await createLocalSession(lastConfig);
    screen = { t: "game", session, local: true };
  }

  function startOnline(name: string) {
    if (screen.t !== "online-entry") return;
    const intent =
      screen.code === null
        ? ({ kind: "create", name } as const)
        : ({ kind: "join", code: screen.code, name } as const);
    screen = { t: "online", session: new OnlineSession(intent) };
  }

  function leaveOnline() {
    if (screen.t === "online") screen.session.destroy();
    goHome();
  }
</script>

{#if screen.t === "home"}
  <Home
    onPassPlay={() => (screen = { t: "setup", mode: "pass" })}
    onBots={() => (screen = { t: "setup", mode: "bots" })}
    onCreateRoom={() => (screen = { t: "online-entry", code: null })}
    onJoinRoom={(code) => (screen = { t: "online-entry", code })}
  />
{:else if screen.t === "setup"}
  <LocalSetup mode={screen.mode} onStart={startLocal} onBack={goHome} />
{:else if screen.t === "game"}
  <Table
    session={screen.session}
    onExit={exitGame}
    onRematch={screen.local ? rematch : undefined}
  />
{:else if screen.t === "online-entry"}
  <OnlineEntry code={screen.code} onSubmit={startOnline} onBack={goHome} />
{:else if screen.t === "online"}
  {@const session = screen.session}
  {#if session.fatal}
    <div class="fatal">
      <h2>Can't reach that table</h2>
      <p>{session.fatal}</p>
      <button class="primary" onclick={leaveOnline}>Back to the clubhouse</button>
    </div>
  {:else if session.started}
    <Table
      session={session}
      onExit={leaveOnline}
      onRematch={session.isHost
        ? () => session.startMatch(session.view?.holes_total ?? 9)
        : undefined}
    />
  {:else}
    <Lobby {session} onLeave={leaveOnline} />
  {/if}
{/if}

<style>
  .fatal {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    text-align: center;
    padding: 2rem;
  }
  .fatal h2 {
    font-family: var(--display, serif);
    color: var(--cream, #f6f0dd);
    margin: 0;
  }
  .fatal p {
    color: rgba(246, 240, 221, 0.7);
    margin: 0;
  }
</style>
