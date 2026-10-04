import { Text, tokens } from "@fluentui/react-components";
import type { ScheduleEntry } from "../api";
import { stepPoints } from "../lib/schedule";

const W = 480;
const H = 120;
const PAD = { left: 28, right: 8, top: 8, bottom: 20 };
const x = (minute: number) => PAD.left + (minute / 1440) * (W - PAD.left - PAD.right);
const y = (brightness: number) => PAD.top + (1 - brightness / 100) * (H - PAD.top - PAD.bottom);
const labelStyle = { fill: tokens.colorNeutralForeground3, fontSize: 9 };

type Props = { entries: ScheduleEntry[]; nowMinutes: number };

/** 一日の輝度の推移（表示専用） */
export function ScheduleChart({ entries, nowMinutes }: Props) {
  const points = stepPoints(entries);
  if (points.length === 0) return <Text>スケジュールがありません</Text>;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="一日の輝度の推移">
      {[0, 50, 100].map((b) => (
        <g key={b}>
          <line x1={PAD.left} x2={W - PAD.right} y1={y(b)} y2={y(b)} style={{ stroke: tokens.colorNeutralStroke2 }} />
          <text x={PAD.left - 4} y={y(b) + 3} textAnchor="end" style={labelStyle}>
            {b}
          </text>
        </g>
      ))}
      {[0, 6, 12, 18, 24].map((h) => (
        <text key={h} x={x(h * 60)} y={H - 6} textAnchor="middle" style={labelStyle}>
          {h}
        </text>
      ))}
      <polyline
        points={points.map((p) => `${x(p.minute)},${y(p.brightness)}`).join(" ")}
        style={{ fill: "none", stroke: tokens.colorBrandForeground1, strokeWidth: 2 }}
      />
      <line
        x1={x(nowMinutes)}
        x2={x(nowMinutes)}
        y1={PAD.top}
        y2={H - PAD.bottom}
        style={{ stroke: tokens.colorPaletteMarigoldForeground1, strokeDasharray: "3 3" }}
      />
    </svg>
  );
}
