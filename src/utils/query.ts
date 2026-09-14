export function routeFromSearch(search: string): { type: 'widget'; id: string } | { type: 'settings' } | { type: 'home' } {
  const params = new URLSearchParams(search);
  const widget = params.get('widget');
  if (widget) return { type: 'widget', id: widget };
  if (params.get('settings') === '1') return { type: 'settings' };
  return { type: 'home' };
}
