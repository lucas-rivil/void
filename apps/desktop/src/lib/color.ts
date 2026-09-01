const PALETTE = [
  "#f2f2f2",
  "#dcdcdc",
  "#c6c6c6",
  "#b0b0b0",
  "#9a9a9a",
  "#848484",
  "#6e6e6e",
  "#e9e9e9",
];

export function authorColor(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i += 1) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  }
  return PALETTE[hash % PALETTE.length];
}
