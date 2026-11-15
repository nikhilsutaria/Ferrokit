// "Only the latest one counts": for async work that a newer request can overtake, such as reading
// a second file before the first has finished.

/** Each call starts a new request and returns a check that stays true only until the next call. */
export function latest(): () => () => boolean {
  let n = 0;
  return () => {
    const mine = ++n;
    return () => mine === n;
  };
}
