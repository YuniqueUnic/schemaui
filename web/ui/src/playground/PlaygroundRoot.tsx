import { useCallback, useState } from "react";
import App from "../App";
import { createWasmTransport } from "../transport/wasmTransport";
import type { SchemaUiTransport } from "../transport/types";
import type { JsonValue } from "../types";
import { SchemaPasteScreen } from "./SchemaPasteScreen";

/**
 * The Playground's whole state machine: paste a schema, or edit the form it
 * produced — plus the raw text of the last paste, kept around so "Back" from
 * the built form means picking up where you left off rather than retyping it.
 */
export function PlaygroundRoot() {
  const [transport, setTransport] = useState<SchemaUiTransport | null>(null);
  const [rawSchemaText, setRawSchemaText] = useState("");
  const [rawDataText, setRawDataText] = useState("");

  const handleReady = useCallback(
    (
      schema: JsonValue,
      defaults: JsonValue,
      schemaText: string,
      dataText: string,
    ) => {
      setRawSchemaText(schemaText);
      setRawDataText(dataText);
      setTransport(createWasmTransport(schema, defaults));
    },
    [],
  );

  const handleBack = useCallback(() => {
    setTransport(null);
  }, []);

  if (!transport) {
    return (
      <SchemaPasteScreen
        onReady={handleReady}
        initialSchemaText={rawSchemaText}
        initialDataText={rawDataText}
      />
    );
  }

  return <App transport={transport} onBack={handleBack} />;
}
