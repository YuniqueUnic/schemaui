import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "sonner";
import "./styles/globals.css";
import App from "./App.tsx";
import { ThemeProvider } from "./theme";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <App />
      <Toaster richColors position="bottom-right" />
    </ThemeProvider>
  </StrictMode>,
);
