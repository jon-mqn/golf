import type { Card } from "./protocol/Card";
import type { Rank } from "./protocol/Rank";
import type { Suit } from "./protocol/Suit";

export const RANK_SYMBOL: Record<Rank, string> = {
  Ace: "A",
  Two: "2",
  Three: "3",
  Four: "4",
  Five: "5",
  Six: "6",
  Seven: "7",
  Eight: "8",
  Nine: "9",
  Ten: "10",
  Jack: "J",
  Queen: "Q",
  King: "K",
};

export const SUIT_SYMBOL: Record<Suit, string> = {
  Clubs: "♣",
  Diamonds: "♦",
  Hearts: "♥",
  Spades: "♠",
};

export function isRed(suit: Suit): boolean {
  return suit === "Hearts" || suit === "Diamonds";
}

export function cardLabel(card: Card): string {
  return `${RANK_SYMBOL[card.rank]}${SUIT_SYMBOL[card.suit]}`;
}
