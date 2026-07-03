import type { Action } from "../protocol/Action";
import type { ClientMsg } from "../protocol/ClientMsg";
import type { Difficulty } from "../protocol/Difficulty";
import type { Event } from "../protocol/Event";
import type { LobbyState } from "../protocol/LobbyState";
import type { PlayerView } from "../protocol/PlayerView";
import type { ServerMsg } from "../protocol/ServerMsg";
import type { GameSession } from "./types";

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

  private ws: WebSocket | null = null;
  private token: string | null = null;
  private intent: OnlineIntent;
  private closing = false;
  private retries = 0;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;

  constructor(intent: OnlineIntent) {
    this.intent = intent;
    if (intent.kind === "rejoin") {
      this.code = intent.code;
      this.token = intent.token;
    }
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

  removeSeat(seat: number) {
    this.sendMsg({ type: "RemoveSeat", seat });
  }

  startMatch(holes: number) {
    this.sendMsg({ type: "StartMatch", holes });
  }

  destroy() {
    this.closing = true;
    if (this.retryTimer !== null) clearTimeout(this.retryTimer);
    if (this.pingTimer !== null) clearInterval(this.pingTimer);
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
      // Keepalive so idle proxies don't drop the socket mid-game.
      if (this.pingTimer !== null) clearInterval(this.pingTimer);
      this.pingTimer = setInterval(() => this.sendMsg({ type: "Ping" }), 30_000);
    };

    ws.onmessage = (raw) => {
      const msg: ServerMsg = JSON.parse(raw.data);
      this.handle(msg);
    };

    ws.onclose = () => {
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
        break;
      case "Events":
        this.events = msg.events;
        break;
      case "State":
        this.view = msg.view;
        break;
      case "Error":
        if (msg.code === "BadToken" || msg.code === "RoomNotFound") {
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
