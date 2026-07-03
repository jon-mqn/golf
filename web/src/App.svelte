<script lang="ts">
  import type { MatchConfig } from "./lib/protocol/MatchConfig";
  import type { GameSession } from "./lib/session/types";
  import { createLocalSession } from "./lib/session/local.svelte";
  import Home from "./lib/components/Home.svelte";
  import LocalSetup from "./lib/components/LocalSetup.svelte";
  import Table from "./lib/components/Table.svelte";

  type Screen =
    | { t: "home" }
    | { t: "setup"; mode: "pass" | "bots" }
    | { t: "game"; session: GameSession; local: boolean };

  let screen = $state<Screen>({ t: "home" });
  let lastConfig: MatchConfig | null = null;

  async function startLocal(config: MatchConfig) {
    lastConfig = config;
    const session = await createLocalSession(config);
    screen = { t: "game", session, local: true };
  }

  function exitGame() {
    if (screen.t === "game") screen.session.destroy();
    screen = { t: "home" };
  }

  async function rematch() {
    if (screen.t !== "game" || !lastConfig) return;
    screen.session.destroy();
    const session = await createLocalSession(lastConfig);
    screen = { t: "game", session, local: true };
  }

  // Online play (rooms) lands with the server milestone.
  function createRoom() {
    alert("Online tables aren't open yet — play on this device for now.");
  }
  function joinRoom(_code: string) {
    createRoom();
  }
</script>

{#if screen.t === "home"}
  <Home
    onPassPlay={() => (screen = { t: "setup", mode: "pass" })}
    onBots={() => (screen = { t: "setup", mode: "bots" })}
    onCreateRoom={createRoom}
    onJoinRoom={joinRoom}
  />
{:else if screen.t === "setup"}
  <LocalSetup mode={screen.mode} onStart={startLocal} onBack={() => (screen = { t: "home" })} />
{:else if screen.t === "game"}
  <Table
    session={screen.session}
    onExit={exitGame}
    onRematch={screen.local ? rematch : undefined}
  />
{/if}
