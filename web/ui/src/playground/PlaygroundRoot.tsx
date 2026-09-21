import { useCallback, useState } from "react";
import App from "../App";
import { createWasmTransport } from "../transport/wasmTransport";
import type { SchemaUiTransport } from "../transport/types";
import type { JsonValue } from "../types";
import { SchemaPasteScreen } from "./SchemaPasteScreen";

/**
 * The Playground's whole state machine: paste a schema, or edit the form it
 * produced. Two states, no way back from the second to the first short of a
 * reload — see `SchemaPasteScreen`'s doc comment for why.
 */
export function PlaygroundRoot() {
  const [transport, setTransport] = useState<SchemaUiTransport | null>(null);

  const handleReady = useCallback((schema: JsonValue, defaults: JsonValue) => {
    setTransport(createWasmTransport(schema, defaults));
  }, []);

  if (!transport) {
    return <SchemaPasteScreen onReady={handleReady} />;
  }

  return <App transport={transport} />;
}
