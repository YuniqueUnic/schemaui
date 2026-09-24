import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "sonner";
import "./styles/globals.css";
import App from "./App.tsx";
import { ThemeProvider } from "./theme";
import { I18nProvider } from "./i18n";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <I18nProvider>
        <App />
        <Toaster richColors position="bottom-right" />
      </I18nProvider>
    </ThemeProvider>
  </StrictMode>,
);
