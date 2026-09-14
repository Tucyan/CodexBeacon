import type { ComponentType } from 'react';
import type { Snapshot, WidgetInstance } from '../types/index';
import { MemoWidget } from './MemoWidget';
import { TodoWidget } from './TodoWidget';
import { CountdownWidget } from './CountdownWidget';
import { CodexUsageWidget } from './CodexUsageWidget';
import { CodexResetWidget } from './CodexResetWidget';

export type WidgetProps = { widget: WidgetInstance; snapshot: Snapshot };
export type WidgetEntry = { id: string; label: string; Component: ComponentType<WidgetProps> };
export const WIDGET_REGISTRY: WidgetEntry[] = [
  { id: 'memo', label: 'Memo', Component: MemoWidget },
  { id: 'todo', label: 'Todo', Component: TodoWidget },
  { id: 'countdown', label: 'Countdown', Component: CountdownWidget },
  { id: 'codex-usage', label: 'Codex Usage', Component: CodexUsageWidget },
  { id: 'codex-reset', label: 'Codex Reset', Component: CodexResetWidget },
];
export function widgetEntry(id: string): WidgetEntry | undefined { return WIDGET_REGISTRY.find((entry) => entry.id === id); }
