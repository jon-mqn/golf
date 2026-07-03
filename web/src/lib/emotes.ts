import type { Emote } from "./protocol/Emote";

/** Display glyph for each emote in the fixed server-side palette. */
export const EMOTE_GLYPH: Record<Emote, string> = {
  Wave: "👋",
  Laugh: "😂",
  Cry: "😭",
  Fire: "🔥",
  Clap: "👏",
  Zzz: "💤",
};

export const ALL_EMOTES = Object.keys(EMOTE_GLYPH) as Emote[];
