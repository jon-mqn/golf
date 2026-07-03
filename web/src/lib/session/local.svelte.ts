import init, { WasmMatch } from "../wasm/golf_wasm.js";
import type { Action } from "../protocol/Action";
import type { Event } from "../protocol/Event";
import type { MatchConfig } from "../protocol/MatchConfig";
import type { PlayerView } from "../protocol/PlayerView";
import type { GameSession } from "./types";

let wasmReady: Promise<unknown> | null = null;

export async function createLocalSession(config: MatchConfig): Promise<LocalSession> {
  wasmReady ??= init();
  await wasmReady;
  return new LocalSession(config);
}

/** Delay between bot sub-actions so humans can follow along. */
const BOT_STEP_MS = 800;

export class LocalSession implements GameSession {
  view = $state<PlayerView | null>(null);
  events = $state<Event[]>([]);
  passTo = $state<string | null>(null);

  private m: WasmMatch;
  private humans: number[];
  private viewSeat: number | null;
  private pendingSeat: number | null = null;
  private botTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(config: MatchConfig) {
    const seed = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
    this.m = new WasmMatch(JSON.stringify(config), seed);
    this.humans = config.seats.flatMap((s, i) => (s.kind.type === "Human" ? [i] : []));
    this.viewSeat = this.humans[0] ?? null;
    this.refresh();
    this.advance();
  }

  send(action: Action) {
    if (this.viewSeat === null) return;
    const actor = this.m.seat_to_act() ?? this.viewSeat;
    this.events = JSON.parse(this.m.apply(actor, JSON.stringify(action)));
    this.refresh();
    this.advance();
  }

  confirmPass() {
    if (this.pendingSeat === null) return;
    this.viewSeat = this.pendingSeat;
    this.pendingSeat = null;
    this.passTo = null;
    this.refresh();
  }

  destroy() {
    if (this.botTimer !== null) clearTimeout(this.botTimer);
    this.m.free();
  }

  /** Re-derive the rendered view. While the pass gate is up (or in a
   * bots-only match) render the spectator view, which holds no private
   * information. */
  private refresh() {
    const seat = this.pendingSeat !== null ? null : this.viewSeat;
    this.view = JSON.parse(this.m.view(seat));
  }

  private advance() {
    const actor = this.m.seat_to_act();
    if (actor === undefined) return; // hole or match complete
    if (this.m.is_bot(actor)) {
      this.botTimer = setTimeout(() => this.botStep(), BOT_STEP_MS);
      return;
    }
    if (actor !== this.viewSeat && this.humans.length > 1) {
      // Another human is up: block the screen until they confirm, and only
      // render public information in the meantime.
      this.pendingSeat = actor;
      this.passTo = this.view?.seats[actor]?.name ?? "the next player";
      this.refresh();
    }
  }

  private botStep() {
    this.botTimer = null;
    this.events = JSON.parse(this.m.bot_step());
    this.refresh();
    this.advance();
  }
}
