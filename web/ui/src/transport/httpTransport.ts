import {
  exitSession,
  fetchSession,
  persistData,
  renderDiagram,
  renderPreview,
  validateData,
} from "../api";
import type { SchemaUiTransport } from "./types";

/** Talks to a running `schemaui` web session over `/api/v1/*`. */
export const httpTransport: SchemaUiTransport = {
  kind: "http",
  bootstrap: fetchSession,
  validate: validateData,
  preview: renderPreview,
  renderContent: renderDiagram,
  async save(data) {
    await persistData(data);
  },
  async exit(data, commit) {
    await exitSession(data, commit);
  },
};
