import React from "react";
import ReactDOM from "react-dom/client";
import { FluentProvider, webDarkTheme, webLightTheme } from "@fluentui/react-components";
import App from "./App";
import { useIsDark } from "./hooks/useIsDark";

function Root() {
  const isDark = useIsDark();
  return (
    <FluentProvider theme={isDark ? webDarkTheme : webLightTheme} style={{ minHeight: "100vh" }}>
      <App />
    </FluentProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
