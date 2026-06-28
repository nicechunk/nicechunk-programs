export function blockKey(x, y, z) {
  return `${x},${y},${z}`;
}

export function chunkKey(x, z) {
  return `${x},${z}`;
}

export function parseCellKey(key) {
  const firstComma = key.indexOf(",");
  const secondComma = key.indexOf(",", firstComma + 1);
  if (firstComma < 0 || secondComma < 0) return [NaN, NaN, NaN];
  return [
    Number(key.slice(0, firstComma)),
    Number(key.slice(firstComma + 1, secondComma)),
    Number(key.slice(secondComma + 1)),
  ];
}
