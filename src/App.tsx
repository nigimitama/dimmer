import { useCallback, useEffect, useState } from "react";
import {
  makeStyles,
  Title3,
  Toast,
  ToastBody,
  Toaster,
  ToastTitle,
  tokens,
  useId,
  useToastController,
} from "@fluentui/react-components";
import { api, type ApplyResult, type Monitor, type Settings } from "./api";
import { BrightnessPanel } from "./components/BrightnessPanel";
import { SchedulePanel } from "./components/SchedulePanel";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
    padding: tokens.spacingHorizontalXL,
    boxSizing: "border-box",
  },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  detail: { whiteSpace: "pre-line" },
});

export default function App() {
  const styles = useStyles();
  const [monitors, setMonitors] = useState<Monitor[] | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const toasterId = useId("toaster");
  const { dispatchToast } = useToastController(toasterId);

  const notifyError = useCallback(
    (title: string, detail?: string) =>
      dispatchToast(
        <Toast>
          <ToastTitle>{title}</ToastTitle>
          {detail && <ToastBody className={styles.detail}>{detail}</ToastBody>}
        </Toast>,
        { intent: "error" },
      ),
    [dispatchToast, styles.detail],
  );

  const refreshMonitors = useCallback(() => {
    api
      .getMonitors()
      .then(setMonitors)
      .catch((e) => notifyError("モニターを取得できませんでした", String(e)));
  }, [notifyError]);

  useEffect(() => {
    refreshMonitors();
    api
      .getSettings()
      .then(setSettings)
      .catch((e) => notifyError("設定を読み込めませんでした", String(e)));
    const unlisten = api.onMonitorsUpdated(setMonitors);
    return () => {
      unlisten.then((f) => f());
    };
  }, [refreshMonitors, notifyError]);

  const reportApply = useCallback(
    (result: ApplyResult) => {
      // 輝度を読み取れないモニター（ノート PC の内蔵ディスプレイなど）への失敗は想定どおりなので知らせない
      const unreadable = new Set((monitors ?? []).filter((m) => m.brightness === null).map((m) => m.id));
      const failed = result.failed.filter((f) => !unreadable.has(f.id));
      if (failed.length > 0) {
        notifyError("一部のモニターに適用できませんでした", failed.map((f) => `${f.name}: ${f.error}`).join("\n"));
      }
    },
    [monitors, notifyError],
  );

  return (
    <div className={styles.root}>
      <header className={styles.header}>
        <Title3>Dimmer</Title3>
        {/* Task 12: SettingsDialog */}
      </header>
      <BrightnessPanel
        monitors={monitors}
        onMonitorsChange={setMonitors}
        onApplied={reportApply}
        onRescan={refreshMonitors}
        onError={notifyError}
      />
      {settings && <SchedulePanel schedule={settings.schedule} onSaved={setSettings} onError={notifyError} />}
      <Toaster toasterId={toasterId} position="bottom" />
    </div>
  );
}
