import { Button, Input, makeStyles, mergeClasses, Text, tokens } from "@fluentui/react-components";
import { DismissRegular } from "@fluentui/react-icons";
import type { DraftEntry, RowErrors } from "../lib/schedule";

const useStyles = makeStyles({
  list: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalXS },
  row: {
    display: "grid",
    gridTemplateColumns: "auto 96px auto",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalS,
    padding: `${tokens.spacingVerticalXS} ${tokens.spacingHorizontalS}`,
    borderRadius: tokens.borderRadiusMedium,
    border: `1px solid transparent`,
  },
  current: { border: `1px solid ${tokens.colorBrandStroke1}` },
  error: { gridColumn: "1 / -1", color: tokens.colorPaletteRedForeground1 },
});

type Props = {
  rows: DraftEntry[];
  errors: RowErrors;
  currentTime: string | null;
  onChange(rows: DraftEntry[]): void;
};

export function ScheduleList({ rows, errors, currentTime, onChange }: Props) {
  const styles = useStyles();
  const patch = (index: number, change: Partial<DraftEntry>) =>
    onChange(rows.map((row, i) => (i === index ? { ...row, ...change } : row)));

  return (
    <div className={styles.list}>
      {rows.map((row, i) => (
        <div key={i} className={mergeClasses(styles.row, row.time === currentTime && styles.current)}>
          <Input type="time" value={row.time} aria-label="時刻" onChange={(_, d) => patch(i, { time: d.value })} />
          <Input
            value={row.brightness}
            inputMode="numeric"
            contentAfter="%"
            aria-label="輝度"
            onChange={(_, d) => patch(i, { brightness: d.value })}
          />
          <Button
            appearance="subtle"
            icon={<DismissRegular />}
            aria-label="削除"
            onClick={() => onChange(rows.filter((_, j) => j !== i))}
          />
          {errors[i] && <Text className={styles.error}>{errors[i]}</Text>}
        </div>
      ))}
    </div>
  );
}
