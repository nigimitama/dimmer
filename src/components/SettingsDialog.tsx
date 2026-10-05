import { useEffect, useState } from "react";
import {
  Button,
  Caption1,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  DialogTrigger,
  makeStyles,
  Switch,
  tokens,
} from "@fluentui/react-components";
import { SettingsRegular } from "@fluentui/react-icons";
import { getVersion } from "@tauri-apps/api/app";
import { api, type Settings } from "../api";

const useStyles = makeStyles({
  content: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalM },
});

type Props = {
  settings: Settings | null;
  onChange(settings: Settings): void;
  onError(title: string, detail?: string): void;
};

export function SettingsDialog({ settings, onChange, onError }: Props) {
  const styles = useStyles();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <Dialog>
      <DialogTrigger disableButtonEnhancement>
        <Button appearance="subtle" icon={<SettingsRegular />} aria-label="Settings" />
      </DialogTrigger>
      <DialogSurface>
        <DialogBody>
          <DialogTitle>Settings</DialogTitle>
          <DialogContent className={styles.content}>
            <Switch
              label="Launch at login"
              checked={settings?.autostart ?? false}
              disabled={settings === null}
              onChange={(_, data) =>
                api
                  .setAutostart(data.checked)
                  .then(onChange)
                  .catch((e) => onError("Couldn't change autostart setting", String(e)))
              }
            />
            <Caption1>Version {version}</Caption1>
          </DialogContent>
          <DialogActions>
            <DialogTrigger disableButtonEnhancement>
              <Button appearance="secondary">Close</Button>
            </DialogTrigger>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
