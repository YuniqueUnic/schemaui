import type { SessionResponse as ServerSessionResponse } from "@schemaui/types/SessionResponse";
import { PrecompiledSession } from "./session_snapshot";

// Type-level guard: if the generated snapshot ever drifts from the
// server-side SessionResponse shape, `pnpm typecheck` will fail here.
export const snapshotTypecheck: ServerSessionResponse = PrecompiledSession;

export {};
