import { useEffect, useState } from "react";

const nowMinutes = () => {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
};

/** 0:00 からの経過分。30秒ごとに更新する */
export function useNowMinutes(): number {
  const [minutes, setMinutes] = useState(nowMinutes);
  useEffect(() => {
    const timer = setInterval(() => setMinutes(nowMinutes()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return minutes;
}
