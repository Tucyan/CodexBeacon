import { useEffect, useState } from 'react';
import { getAutostartStatus, setAutostartEnabled } from '../runtime/tauri';
import type { AutostartStatus } from '../types/index';
import { autostartErrorMessage } from './autostart';

export function AutostartSetting() {
  const [status, setStatus] = useState<AutostartStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void getAutostartStatus()
      .then((next) => { if (active) setStatus(next); })
      .catch((cause) => { if (active) setError(autostartErrorMessage(cause)); });
    return () => { active = false; };
  }, []);

  const update = async (enabled: boolean) => {
    setBusy(true);
    setError(null);
    try { setStatus(await setAutostartEnabled(enabled)); }
    catch (cause) { setError(autostartErrorMessage(cause)); }
    finally { setBusy(false); }
  };

  const description = status === null
    ? '正在读取 Windows 启动设置…'
    : !status.enabled
      ? '关闭。仅为当前 Windows 用户设置，不需要管理员权限。'
      : status.pathCurrent
        ? '已启用。登录 Windows 后将从当前程序位置启动。'
        : '已启用，但指向另一个仍然存在的程序副本。';

  return <section className="autostart-setting">
    <label className="toggle"><input type="checkbox" checked={status?.enabled ?? false} disabled={status === null || busy} onChange={(event) => void update(event.target.checked)} /> 开机自动运行</label>
    <p className="muted">{description}</p>
    {status?.enabled && !status.pathCurrent && <button disabled={busy} onClick={() => void update(true)}>改为当前程序</button>}
    {error && <p className="provider-error" role="alert">{error}</p>}
  </section>;
}
