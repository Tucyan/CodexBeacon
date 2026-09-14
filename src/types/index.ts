export type WidgetType = 'memo' | 'todo' | 'countdown' | 'codex-reset' | 'codex-usage';
export type ScaleMode = 'inherit' | 'custom';
export type ShowDesktopMode = 'auto' | 'follow-system';
export type ProviderState = 'idle' | 'loading' | 'ready' | 'offline' | 'error' | 'unavailable';
export interface ThemeConfig { accent: string; background: string; fieldBackground: string; foreground: string; mutedForeground: string; backgroundOpacity: number; contentOpacity: number }
export interface AppSettings { layoutLocked: boolean; globalScale: number; theme: ThemeConfig; showDesktopMode: ShowDesktopMode; fadeEnabled: boolean; backupIntervalDays: number }
export interface BackupStatus { lastBackupAt: number | null; lastBackupPath: string | null }
export interface BackupOperation { status: 'saved' | 'imported' | 'cancelled'; path: string | null; createdAt: number | null }
export interface AutostartStatus { enabled: boolean; pathCurrent: boolean }
export interface WidgetInstance { id: string; type: WidgetType; enabled: boolean; visibleWanted: boolean; x: number; y: number; width: number; height: number; scaleMode: ScaleMode; scale: number; config: Record<string, unknown> }
export interface WidgetGeometry { x: number; y: number; width: number; height: number }
export interface LayoutProfile { workArea: { width: number; height: number }; widgets: Record<string, WidgetGeometry> }
export interface TodoItem { id: string; text: string; completed: boolean }
export interface CountdownItem { id: string; name: string; target: number }
export interface WidgetContent { memo: string; todos: TodoItem[]; countdowns: CountdownItem[] }
export interface ProviderSnapshot { state: ProviderState; lastSuccess: number | null; error: string | null; data: unknown }
export interface Providers { codex: ProviderSnapshot; reset: ProviderSnapshot }
export interface Snapshot { revision: number; settings: AppSettings; widgets: WidgetInstance[]; content: Record<string, WidgetContent>; providers: Providers; activeLayoutKey?: string | null; layoutProfiles?: Record<string, LayoutProfile> }
export type Action =
  | { type: 'settings'; settings: AppSettings }
  | { type: 'widget'; id: string; enabled?: boolean; scaleMode?: ScaleMode; scale?: number }
  | { type: 'content'; id: string; content: WidgetContent }
  | { type: 'hide'; id: string }
  | { type: 'show'; id: string }
  | { type: 'reset-layout'; id?: string }
  | { type: 'retry'; provider: 'codex' | 'reset' };
