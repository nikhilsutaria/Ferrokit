// Small helpers for showing keyboard events.

const LOCATIONS = ['Standard', 'Left', 'Right', 'Numpad'];

/** KeyboardEvent.location as a word. */
export function locationName(location: number): string {
  return LOCATIONS[location] ?? `Unknown (${location})`;
}

/** KeyboardEvent.key, with invisible or cryptic values made readable. */
export function keyLabel(key: string): string {
  if (key === ' ') return 'Space';
  if (key === 'Dead') return 'Dead key';
  return key;
}
