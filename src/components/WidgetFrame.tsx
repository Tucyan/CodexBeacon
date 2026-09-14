import type { ReactNode } from 'react';
import type { WidgetInstance } from '../types/index';
import { startDrag, startResize } from '../runtime/tauri';

const directions = ['TopLeft', 'Top', 'TopRight', 'Right', 'BottomRight', 'Bottom', 'BottomLeft', 'Left'];

export function WidgetFrame({ widget, title, children, onHide, bodyClassName, locked = false }: { widget: WidgetInstance; title: string; children: ReactNode; onHide?: () => void; bodyClassName?: string; locked?: boolean }) {
  return <main className="widget-frame" data-widget-id={widget.id}>
    <header className="widget-header" onPointerDown={() => { if (!locked) void startDrag().catch(() => undefined); }}>
      <span>{title}</span>
      {onHide && <button className="icon-button" aria-label="Hide widget" onPointerDown={(e) => e.stopPropagation()} onClick={onHide}>×</button>}
    </header>
    <section className={`widget-body${bodyClassName ? ` ${bodyClassName}` : ''}`}>{children}</section>
    {!locked && directions.map((direction) => <button key={direction} aria-label={`Resize ${direction}`} className={`resize-handle resize-${direction.toLowerCase()}`} onPointerDown={(event) => { event.preventDefault(); event.stopPropagation(); void startResize(direction).catch(() => undefined); }} />)}
  </main>;
}
