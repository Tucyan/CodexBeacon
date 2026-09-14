import { useEffect, useState } from 'react';
import { WidgetFrame } from '../components/WidgetFrame';
import { dashboardStore } from '../stores/dashboard';
import { formatCountdown, secondsUntil } from '../utils/time';
import type { WidgetProps } from './registry';

function localDateTimeValue(epochSeconds: number): string {
  const date = new Date(epochSeconds * 1000);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function CountdownWidget({ widget, snapshot }: WidgetProps) {
  const content = snapshot.content[widget.id] ?? { memo: '', todos: [], countdowns: [] };
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [name, setName] = useState(''); const [target, setTarget] = useState('');
  useEffect(() => { const timer = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000); return () => clearInterval(timer); }, []);
  const update = (countdowns: typeof content.countdowns) => dashboardStore.editContent(widget.id, { ...content, countdowns });
  return <WidgetFrame widget={widget} locked={snapshot.settings.layoutLocked} title="Countdown" onHide={() => void dashboardStore.send({ type: 'hide', id: widget.id })}>
    <form className="countdown-add" onSubmit={(event) => { event.preventDefault(); const epoch = Date.parse(target) / 1000; if (!name.trim() || !Number.isFinite(epoch)) return; update([...content.countdowns, { id: crypto.randomUUID(), name: name.trim(), target: Math.floor(epoch) }]); setName(''); setTarget(''); }}>
      <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Name" aria-label="Countdown name" /><input type="datetime-local" value={target} onChange={(e) => setTarget(e.target.value)} aria-label="Countdown target" /><button>Add</button>
    </form>
    <ul className="countdown-list">{content.countdowns.map((item) => <li key={item.id}><div><strong>{item.name}</strong><span>{formatCountdown(secondsUntil(item.target, now))}</span></div><input aria-label={`Edit ${item.name}`} value={localDateTimeValue(item.target)} onChange={(e) => { const next = Date.parse(e.target.value) / 1000; if (Number.isFinite(next)) update(content.countdowns.map((entry) => entry.id === item.id ? { ...entry, target: Math.floor(next) } : entry)); }} type="datetime-local" /><button aria-label={`Delete ${item.name}`} onClick={() => update(content.countdowns.filter((entry) => entry.id !== item.id))}>×</button></li>)}</ul>
  </WidgetFrame>;
}
