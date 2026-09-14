import { WidgetFrame } from '../components/WidgetFrame';
import { dashboardStore } from '../stores/dashboard';
import type { WidgetProps } from './registry';

export function MemoWidget({ widget, snapshot }: WidgetProps) {
  const content = snapshot.content[widget.id] ?? { memo: '', todos: [], countdowns: [] };
  return <WidgetFrame widget={widget} locked={snapshot.settings.layoutLocked} bodyClassName="memo-body" title="Memo" onHide={() => void dashboardStore.send({ type: 'hide', id: widget.id })}>
    <textarea className="memo-input" value={content.memo} placeholder="Write a note…" onChange={(event) => dashboardStore.editContent(widget.id, { ...content, memo: event.target.value })} />
  </WidgetFrame>;
}
