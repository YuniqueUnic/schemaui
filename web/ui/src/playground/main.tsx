import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "sonner";
import "../styles/globals.css";
import { ThemeProvider } from "../theme";
import { I18nProvider } from "../i18n";
import { PlaygroundRoot } from "./PlaygroundRoot";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <I18nProvider>
        <PlaygroundRoot />
        <Toaster richColors position="bottom-right" />
      </I18nProvider>
    </ThemeProvider>
  </StrictMode>,
);
