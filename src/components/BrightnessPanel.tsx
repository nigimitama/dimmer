import { Button, Card, Divider, makeStyles, Slider, Spinner, Text, tokens } from "@fluentui/react-components";
import { api, type ApplyResult, type Monitor } from "../api";
import { useDebouncedCallback } from "../hooks/useDebouncedCallback";

const useStyles = makeStyles({
  card: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalS },
  row: { display: "grid", gridTemplateColumns: "140px 1fr 48px", alignItems: "center", gap: tokens.spacingHorizontalM },
  label: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  value: { textAlign: "right" },
  muted: { color: tokens.colorNeutralForeground3 },
  empty: { display: "flex", flexDirection: "column", alignItems: "flex-start", gap: tokens.spacingVerticalS },
});

type RowProps = {
  label: string;
  value: number | null;
  strong?: boolean;
  onChange(value: number): void;
  onCommit(value: number): void;
};

function BrightnessRow({ label, value, strong, onChange, onCommit }: RowProps) {
  const styles = useStyles();
  const commit = useDebouncedCallback(onCommit, 100);
  return (
    <div className={styles.row}>
      <Text className={styles.label} weight={strong ? "semibold" : "regular"} title={label}>
        {label}
      </Text>
      {value === null ? (
        <Text className={styles.muted}>読み取れません</Text>
      ) : (
        <Slider
          min={0}
          max={100}
          step={10}
          value={value}
          aria-label={label}
          onChange={(_, data) => {
            onChange(data.value);
            commit(data.value);
          }}
        />
      )}
      <Text className={styles.value} weight="semibold">
        {value === null ? "—" : `${value}%`}
      </Text>
    </div>
  );
}

type Props = {
  monitors: Monitor[] | null;
  onMonitorsChange(monitors: Monitor[]): void;
  onApplied(result: ApplyResult): void;
  onRescan(): void;
  onError(title: string, detail?: string): void;
};

export function BrightnessPanel({ monitors, onMonitorsChange, onApplied, onRescan, onError }: Props) {
  const styles = useStyles();

  if (monitors === null) return <Spinner label="モニターを検出しています…" />;
  if (monitors.length === 0) {
    return (
      <Card className={styles.empty}>
        <Text>DDC/CI 対応のモニターが見つかりません</Text>
        <Button onClick={onRescan}>再検出</Button>
      </Card>
    );
  }

  const readable = monitors.flatMap((m) => (m.brightness === null ? [] : [m.brightness]));
  const average = readable.length ? Math.round(readable.reduce((a, b) => a + b, 0) / readable.length) : null;
  const fail = (e: unknown) => onError("輝度を変更できませんでした", String(e));

  return (
    <Card className={styles.card}>
      <BrightnessRow
        label="すべて"
        value={average}
        strong
        onChange={(v) => onMonitorsChange(monitors.map((m) => (m.brightness === null ? m : { ...m, brightness: v })))}
        onCommit={(v) => api.setBrightnessAll(v).then(onApplied).catch(fail)}
      />
      <Divider />
      {monitors.map((m) => (
        <BrightnessRow
          key={m.id}
          label={m.name}
          value={m.brightness}
          onChange={(v) => onMonitorsChange(monitors.map((x) => (x.id === m.id ? { ...x, brightness: v } : x)))}
          onCommit={(v) => api.setBrightness(m.id, v).then(onApplied).catch(fail)}
        />
      ))}
    </Card>
  );
}
