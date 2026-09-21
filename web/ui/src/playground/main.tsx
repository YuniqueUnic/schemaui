import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "sonner";
import "../styles/globals.css";
import { ThemeProvider } from "../theme";
import { PlaygroundRoot } from "./PlaygroundRoot";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <PlaygroundRoot />
      <Toaster richColors position="bottom-right" />
    </ThemeProvider>
  </StrictMode>,
);
