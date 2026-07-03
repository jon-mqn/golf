import type { Action } from "../protocol/Action";
import type { Event } from "../protocol/Event";
import type { PlayerView } from "../protocol/PlayerView";

/**
 * One interface for both game modes: `LocalSession` (WASM engine in the
 * browser) and `OnlineSession` (WebSocket to the server). The Table screen
 * only ever talks to this.
 */
export type SessionStatus = "connecting" | "open" | "reconnecting" | "closed";

export interface GameSession {
  /** Redacted view for the active viewer; reactive ($state). */
  readonly view: PlayerView | null;
  /** Events from the most recent state change, for animations. */
  readonly events: Event[];
  /** Pass-and-play: name of the player who must take the device, or null. */
  readonly passTo: string | null;
  /** Online only: connection state and last transient server error. */
  readonly status?: SessionStatus;
  readonly error?: string | null;
  send(action: Action): void;
  /** Pass-and-play: the named player confirmed they have the device. */
  confirmPass(): void;
  destroy(): void;
}
