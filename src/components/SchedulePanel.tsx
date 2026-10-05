import { useState } from "react";
import { Button, Caption1, Card, makeStyles, Subtitle2, tokens } from "@fluentui/react-components";
import { AddRegular } from "@fluentui/react-icons";
import { api, type ScheduleEntry, type Settings } from "../api";
import { useNowMinutes } from "../hooks/useNowMinutes";
import { currentIndex, nextIndex, toDraft, validateDraft, type DraftEntry } from "../lib/schedule";
import { ScheduleChart } from "./ScheduleChart";
import { ScheduleList } from "./ScheduleList";

const useStyles = makeStyles({
  card: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalS },
  heading: { display: "flex", justifyContent: "space-between", alignItems: "baseline" },
  next: { color: tokens.colorBrandForeground1 },
  actions: { display: "flex", gap: tokens.spacingHorizontalS },
});

type Props = {
  schedule: ScheduleEntry[];
  onSaved(settings: Settings): void;
  onError(title: string, detail?: string): void;
};

export function SchedulePanel({ schedule, onSaved, onError }: Props) {
  const styles = useStyles();
  const [rows, setRows] = useState<DraftEntry[]>(() => toDraft(schedule));
  const nowMinutes = useNowMinutes();
  const { errors } = validateDraft(rows);
  const fail = (e: unknown) => onError("スケジュールを保存できませんでした", String(e));

  // 編集のたびに自動保存する。エラーがある間は保存しない
  const update = (next: DraftEntry[]) => {
    setRows(next);
    const { entries } = validateDraft(next);
    if (entries) api.saveSchedule(entries).then(onSaved).catch(fail);
  };

  const reset = () =>
    api
      .resetSchedule()
      .then((settings) => {
        setRows(toDraft(settings.schedule));
        onSaved(settings);
      })
      .catch(fail);

  const current = currentIndex(schedule, nowMinutes);
  const next = nextIndex(schedule, nowMinutes);

  return (
    <Card className={styles.card}>
      <div className={styles.heading}>
        <Subtitle2>スケジュール</Subtitle2>
        {next !== null && (
          <Caption1 className={styles.next}>
            次 {schedule[next].time} → {schedule[next].brightness}%
          </Caption1>
        )}
      </div>
      <ScheduleChart entries={schedule} nowMinutes={nowMinutes} />
      <ScheduleList
        rows={rows}
        errors={errors}
        currentTime={current === null ? null : schedule[current].time}
        onChange={update}
      />
      <div className={styles.actions}>
        <Button icon={<AddRegular />} onClick={() => update([...rows, { time: "00:00", brightness: "50" }])}>
          追加
        </Button>
        <Button onClick={reset}>デフォルトに戻す</Button>
      </div>
    </Card>
  );
}
