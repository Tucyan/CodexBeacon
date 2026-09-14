import { useState } from 'react';
import { WidgetFrame } from '../components/WidgetFrame';
import { dashboardStore } from '../stores/dashboard';
import type { WidgetProps } from './registry';

export function TodoWidget({ widget, snapshot }: WidgetProps) {
  const content = snapshot.content[widget.id] ?? { memo: '', todos: [], countdowns: [] };
  const [text, setText] = useState('');
  const update = (todos: typeof content.todos) => dashboardStore.editContent(widget.id, { ...content, todos });
  return <WidgetFrame widget={widget} locked={snapshot.settings.layoutLocked} title="Todo" onHide={() => void dashboardStore.send({ type: 'hide', id: widget.id })}>
    <form className="todo-add" onSubmit={(event) => { event.preventDefault(); const value = text.trim(); if (!value) return; update([...content.todos, { id: crypto.randomUUID(), text: value, completed: false }]); setText(''); }}>
      <input value={text} onChange={(e) => setText(e.target.value)} placeholder="Add a task…" aria-label="New task" /><button>Add</button>
    </form>
    <ul className="todo-list">{content.todos.map((todo) => <li key={todo.id} className={todo.completed ? 'completed' : ''}><input type="checkbox" checked={todo.completed} aria-label={`Complete ${todo.text}`} onChange={() => update(content.todos.map((item) => item.id === todo.id ? { ...item, completed: !item.completed } : item))} /><input value={todo.text} onChange={(e) => update(content.todos.map((item) => item.id === todo.id ? { ...item, text: e.target.value } : item))} /><button type="button" aria-label={`Delete ${todo.text}`} onClick={() => update(content.todos.filter((item) => item.id !== todo.id))}>×</button></li>)}</ul>
  </WidgetFrame>;
}
