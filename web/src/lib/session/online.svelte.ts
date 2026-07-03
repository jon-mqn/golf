import type { Action } from "../protocol/Action";
import type { ClientMsg } from "../protocol/ClientMsg";
import type { Difficulty } from "../protocol/Difficulty";
import type { Emote } from "../protocol/Emote";
import type { Event } from "../protocol/Event";
import type { LobbyState } from "../protocol/LobbyState";
import type { PlayerView } from "../protocol/PlayerView";
import type { ServerMsg } from "../protocol/ServerMsg";
import type { GameSession } from "./types";

/** Ping cadence; the server answers each with a Pong. */
const PING_MS = 20_000;
/** No server traffic for this long ⇒ the socket is half-dead: reconnect. */
const STALE_MS = 50_000;
/** How long an emote bubble stays up. */
const EMOTE_MS = 2_600;

export type OnlineIntent =
  | { kind: "create"; name: string }
  | { kind: "join"; code: string; name: string }
  | { kind: "rejoin"; code: string; token: string };

export type OnlineStatus = "connecting" | "open" | "reconnecting" | "closed";

const tokenKey = (code: string) => `golf:room:${code}`;

export function savedToken(code: string): string | null {
  return localStorage.getItem(tokenKey(code));
}

export class OnlineSession implements GameSession {
  view = $state<PlayerView | null>(null);
  events = $state<Event[]>([]);
  readonly passTo = null;

  lobby = $state<LobbyState | null>(null);
  status = $state<OnlineStatus>("connecting");
  /** Transient server error for a toast; cleared on the next message. */
  error = $state<string | null>(null);
  /** Set when the room is unreachable and retrying is pointless. */
  fatal = $state<string | null>(null);
  code = $state<string | null>(null);
  seat = $state<number | null>(null);
  /** Latest reaction per seat; `n` disambiguates repeats of the same emote. */
  emotes = $state<Record<number, { emote: Emote; n: number }>>({});

  private ws: WebSocket | null = null;
  private token: string | null = null;
  private intent: OnlineIntent;
  private closing = false;
  private retries = 0;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private lastMsgAt = Date.now();
  private emoteN = 0;
  private emoteTimers: Record<number, ReturnType<typeof setTimeout>> = {};
  private wake = () => this.onWake();

  constructor(intent: OnlineIntent) {
    this.intent = intent;
    if (intent.kind === "rejoin") {
      this.code = intent.code;
      this.token = intent.token;
    }
    // Phones freeze timers and kill sockets in the background; retry the
    // moment we're visible/online again instead of waiting out the backoff.
    window.addEventListener("online", this.wake);
    document.addEventListener("visibilitychange", this.wake);
    this.connect();
  }

  get isHost(): boolean {
    return this.seat !== null && this.seat === (this.lobby?.host ?? 0);
  }

  get started(): boolean {
    return this.view !== null;
  }

  send(action: Action) {
    this.sendMsg({ type: "Act", action });
  }

  confirmPass() {}

  addBot(difficulty: Difficulty) {
    this.sendMsg({ type: "AddBot", difficulty });
  }

  /** In the lobby: frees the seat. Mid-game: hands the seat to a bot. */
  removeSeat(seat: number) {
    this.sendMsg({ type: "RemoveSeat", seat });
  }

  startMatch(holes: number) {
    this.sendMsg({ type: "StartMatch", holes });
  }

  sendEmote(emote: Emote) {
    this.sendMsg({ type: "Emote", emote });
  }

  destroy() {
    this.closing = true;
    window.removeEventListener("online", this.wake);
    document.removeEventListener("visibilitychange", this.wake);
    if (this.retryTimer !== null) clearTimeout(this.retryTimer);
    if (this.pingTimer !== null) clearInterval(this.pingTimer);
    for (const t of Object.values(this.emoteTimers)) clearTimeout(t);
    this.ws?.close();
  }

  private connect() {
    const proto = location.protocol === "https:" ? "wss" : "ws";
    const ws = new WebSocket(`${proto}://${location.host}/ws`);
    this.ws = ws;

    ws.onopen = () => {
      const hello: ClientMsg =
        this.token && this.code
          ? { type: "Rejoin", code: this.code, token: this.token }
          : this.intent.kind === "create"
            ? { type: "CreateRoom", name: this.intent.name }
            : this.intent.kind === "join"
              ? { type: "JoinRoom", code: this.intent.code, name: this.intent.name }
              : { type: "Rejoin", code: this.intent.code, token: this.intent.token };
      ws.send(JSON.stringify(hello));
      // Keepalive so idle proxies don't drop the socket mid-game, plus a
      // staleness check: a half-dead socket never errors, it just goes
      // quiet — force a close so the reconnect loop takes over.
      this.lastMsgAt = Date.now();
      if (this.pingTimer !== null) clearInterval(this.pingTimer);
      this.pingTimer = setInterval(() => {
        if (Date.now() - this.lastMsgAt > STALE_MS) {
          ws.close();
          return;
        }
        this.sendMsg({ type: "Ping" });
      }, PING_MS);
    };

    ws.onmessage = (raw) => {
      this.lastMsgAt = Date.now();
      const msg: ServerMsg = JSON.parse(raw.data);
      this.handle(msg);
    };

    ws.onclose = () => {
      if (this.pingTimer !== null) clearInterval(this.pingTimer);
      this.pingTimer = null;
      if (this.closing) {
        this.status = "closed";
        return;
      }
      // Reconnect only once we hold a seat token; otherwise surface failure.
      if (this.token && this.code && !this.fatal) {
        this.status = "reconnecting";
        const delay = Math.min(1000 * 2 ** this.retries, 10_000);
        this.retries += 1;
        this.retryTimer = setTimeout(() => this.connect(), delay);
      } else {
        this.status = "closed";
        this.fatal ??= "Connection lost.";
      }
    };
  }

  /** Network came back or the tab woke up: don't wait out the backoff. */
  private onWake() {
    if (this.closing || this.fatal) return;
    if (document.visibilityState !== "visible") return;
    if (this.ws?.readyState === WebSocket.OPEN) {
      // The socket may be a zombie after a phone nap: close it right away
      // if it's already stale, otherwise probe it and let the staleness
      // check in the ping loop reap it if the pong never comes.
      if (Date.now() - this.lastMsgAt > STALE_MS) this.ws.close();
      else this.sendMsg({ type: "Ping" });
      return;
    }
    if (this.ws?.readyState === WebSocket.CONNECTING) return;
    if (this.token && this.code) {
      if (this.retryTimer !== null) clearTimeout(this.retryTimer);
      this.status = "reconnecting";
      this.connect();
    }
  }

  private handle(msg: ServerMsg) {
    switch (msg.type) {
      case "RoomJoined":
        this.status = "open";
        this.retries = 0;
        this.code = msg.code;
        this.seat = msg.seat;
        this.token = msg.token;
        localStorage.setItem(tokenKey(msg.code), msg.token);
        history.replaceState(null, "", `/room/${msg.code}`);
        break;
      case "Lobby":
        this.lobby = msg.state;
        // Seats can shift when the host removes one; the lobby knows ours.
        this.seat = msg.state.you;
        break;
      case "Events":
        this.events = msg.events;
        break;
      case "State":
        this.view = msg.view;
        break;
      case "Emote": {
        const n = ++this.emoteN;
        this.emotes = { ...this.emotes, [msg.seat]: { emote: msg.emote, n } };
        clearTimeout(this.emoteTimers[msg.seat]);
        this.emoteTimers[msg.seat] = setTimeout(() => {
          if (this.emotes[msg.seat]?.n === n) {
            const { [msg.seat]: _, ...rest } = this.emotes;
            this.emotes = rest;
          }
        }, EMOTE_MS);
        break;
      }
      case "Error":
        if (msg.code === "Kicked") {
          if (this.code) localStorage.removeItem(tokenKey(this.code));
          this.token = null;
          this.fatal = "The host removed you from the table.";
          this.closing = true;
          this.ws?.close();
          this.status = "closed";
        } else if (msg.code === "BadToken" || msg.code === "RoomNotFound") {
          // The room is gone (or our token is stale): stop retrying.
          if (this.code) localStorage.removeItem(tokenKey(this.code));
          this.token = null;
          this.fatal =
            msg.code === "BadToken"
              ? "Your seat at this table is no longer valid."
              : "That table doesn't exist (or has closed).";
          this.closing = true;
          this.ws?.close();
          this.status = "closed";
        } else {
          this.error = msg.message;
        }
        break;
      case "Pong":
        break;
    }
  }

  private sendMsg(msg: ClientMsg) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }
}
