import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import Search from './Search.svelte';

// SvelteKit moves focus to the body after every navigation, including the first one, unless an
// element has an `autofocus` attribute: then it focuses that element instead. Focusing the box from
// code alone loses that race, so the home page's search must carry the attribute.
describe('Search', () => {
  it('marks the hero search box autofocus, so it keeps focus once the page has loaded', () => {
    const { body } = render(Search, { props: { hero: true, autofocus: true } });
    expect(body).toMatch(/<input[^>]*\sautofocus/);
  });

  it('leaves the header search box alone', () => {
    const { body } = render(Search, { props: {} });
    expect(body).not.toMatch(/<input[^>]*\sautofocus/);
  });
});
