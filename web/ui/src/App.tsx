import { useEffect, useMemo, useRef, useState } from 'react';
import { AppHeader } from './components/AppHeader';
import { NodeRenderer } from './components/NodeRenderer';
import { PreviewPane } from './components/PreviewPane';
import { StatusBar } from './components/StatusBar';
import { ShortcutBar } from './components/ShortcutBar';
import { TreeView } from './components/TreeView';
import { OverlayProvider } from './components/Overlay';
import {
  exitSession,
  fetchSession,
  persistData,
  renderPreview,
  validateData,
} from './api';
import type { JsonValue, SessionResponse, UiNode, UiAst } from './types';
import { applyUiDefaults } from './ui-ast';
import { deepClone, getPointerValue, setPointerValue } from './utils/jsonPointer';
import { useResizableColumns } from './hooks/useResizableColumns';
import './index.css';

export default function App() {
  const [session, setSession] = useState<SessionResponse | null>(null);
  const [data, setData] = useState<JsonValue>({});
  const [formats, setFormats] = useState<string[]>(['json']);
  const [previewFormat, setPreviewFormat] = useState('json');
  const [previewPretty, setPreviewPretty] = useState(true);
  const [previewPayload, setPreviewPayload] = useState('{}');
  const [errors, setErrors] = useState<Map<string, string>>(new Map());
  const [dirty, setDirty] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [exiting, setExiting] = useState(false);
  const [status, setStatus] = useState('Loading schema…');
  const [selectedPointer, setSelectedPointer] = useState<string>('');
  const validationSeq = useRef(0);
  const previewSeq = useRef(0);
  const { sizes, startDrag } = useResizableColumns({ nav: 280, preview: 380 });

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const payload = await fetchSession();
        if (!mounted) return;
        const withDefaults = applyUiDefaults(payload.ui_ast, payload.data ?? {});
        setSession(payload);
        setData(withDefaults);
        setSelectedPointer(resolveInitialPointer(payload.ui_ast));
        const availableFormats =
          payload.formats?.length && payload.formats.length > 0 ? payload.formats : ['json'];
        setFormats(availableFormats);
        setPreviewFormat(availableFormats.includes('json') ? 'json' : availableFormats[0]);
        setStatus('Ready');
        await Promise.all([
          runValidation(withDefaults),
          updatePreview(withDefaults, previewPretty, availableFormats[0]),
        ]);
      } catch (error) {
        console.error(error);
        setStatus('Failed to load session');
      } finally {
        if (mounted) setLoading(false);
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  const runValidation = async (value: JsonValue) => {
    const seq = ++validationSeq.current;
    try {
      const result = await validateData(value);
      if (seq !== validationSeq.current) return;
      const next = new Map<string, string>();
      result.errors?.forEach((err) => next.set(err.pointer || '', err.message));
      setErrors(next);
    } catch (error) {
      console.error('validate failed', error);
    }
  };

  const updatePreview = async (value: JsonValue, pretty: boolean, format: string) => {
    const seq = ++previewSeq.current;
    try {
      const result = await renderPreview(value, format, pretty);
      if (seq !== previewSeq.current) return;
      setPreviewPayload(result.payload);
    } catch (error) {
      console.error('preview failed', error);
    }
  };

  const handleChange = (pointer: string, value: JsonValue) => {
    setData((prev) => {
      const next = setPointerValue(prev, pointer, deepClone(value));
      runValidation(next);
      updatePreview(next, previewPretty, previewFormat);
      setDirty(true);
      return next;
    });
  };

  const handleSave = async () => {
    if (!session) return;
    setSaving(true);
    try {
      await persistData(data);
      setDirty(false);
    } finally {
      setSaving(false);
    }
  };

  const handleExit = async () => {
    setExiting(true);
    try {
      await exitSession(data, true);
    } finally {
      setExiting(false);
    }
  };

  const selectedNode = useMemo(
    () => findNodeByPointer(session?.ui_ast?.roots ?? [], selectedPointer),
    [session, selectedPointer],
  );

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#0b1121] text-slate-200">
        Loading session…
      </div>
    );
  }

  return (
    <OverlayProvider>
    <div className="flex h-screen flex-col bg-[#0b1121] text-slate-100">
      <AppHeader
        title={session?.title ?? session?.ui_ast?.roots[0]?.title}
        description={session?.ui_ast?.roots[0]?.description}
        dirty={dirty}
        saving={saving}
        exiting={exiting}
        onSave={handleSave}
        onExit={handleExit}
      />
      <div className="flex flex-1 overflow-hidden border-t border-b border-slate-800/70 bg-gradient-to-br from-slate-900 via-slate-950 to-[#0b1121]">
        <aside
          className="border-r border-slate-800/70 backdrop-blur"
          style={{ width: sizes.nav }}
        >
          <div className="flex items-center justify-between px-4 py-3 text-xs uppercase tracking-[0.3em] text-slate-500">
            Schema
          </div>
          <TreeView
            ast={session?.ui_ast}
            selectedPointer={selectedPointer}
            onSelect={(pointer) => setSelectedPointer(pointer)}
          />
        </aside>
        <div
          className="w-1 cursor-col-resize bg-transparent"
          onPointerDown={(event) => startDrag(event, 'nav')}
        />
        <main className="flex flex-1 flex-col overflow-hidden px-6 py-4">
          <EditorBreadcrumbs node={selectedNode} pointer={selectedPointer} />
          <div className="mt-4 flex-1 overflow-y-auto pr-4 text-sm">
            {selectedNode ? (
              <EditorBody
                node={selectedNode}
                data={data}
                errors={errors}
                onChange={handleChange}
              />
            ) : (
              <div className="text-center text-slate-500">
                Select a node from the tree to start editing.
              </div>
            )}
          </div>
        </main>
        <div
          className="w-1 cursor-col-resize bg-transparent"
          onPointerDown={(event) => startDrag(event, 'preview')}
        />
        <section
          className="flex h-full flex-col border-l border-slate-800/70 backdrop-blur"
          style={{ width: sizes.preview }}
        >
          <PreviewPane
            formats={formats}
            format={previewFormat}
            onFormatChange={(fmt) => {
              setPreviewFormat(fmt);
              updatePreview(data, previewPretty, fmt);
            }}
            pretty={previewPretty}
            onPrettyChange={(pretty) => {
              setPreviewPretty(pretty);
              updatePreview(data, pretty, previewFormat);
            }}
            payload={previewPayload}
            loading={false}
          />
        </section>
      </div>
      <ShortcutBar />
      <StatusBar status={status} dirty={dirty} validating={false} errorCount={errors.size} />
    </div>
    </OverlayProvider>
  );
}

function findNodeByPointer(nodes: UiNode[], pointer?: string): UiNode | undefined {
  if (!pointer) return nodes[0];
  for (const node of nodes) {
    if (node.pointer === pointer) {
      return node;
    }
    if (node.kind.type === 'object') {
      const child = findNodeByPointer(node.kind.children ?? [], pointer);
      if (child) return child;
    }
  }
  return undefined;
}

function resolveInitialPointer(ast?: UiAst | null): string {
  if (!ast || !ast.roots.length) return '';
  const root = ast.roots[0];
  if (root.kind.type === 'object' && root.kind.children?.length) {
    return root.kind.children[0].pointer;
  }
  return root.pointer;
}

function EditorBreadcrumbs({ node, pointer }: { node?: UiNode; pointer?: string }) {
  const segments = pointerSegments(pointer);
  if (!segments.length) {
    return null;
  }
  return (
    <nav className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
      {segments.map((segment, index) => (
        <span key={`${segment}-${index}`} className="rounded-full bg-white/5 px-3 py-1 text-slate-200">
          {segment}
        </span>
      ))}
      {node?.required ? (
        <span className="text-[10px] uppercase tracking-[0.3em] text-rose-400">Required</span>
      ) : null}
    </nav>
  );
}

function EditorBody({
  node,
  data,
  errors,
  onChange,
}: {
  node: UiNode;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}) {
  if (node.kind.type === 'object') {
    return (
      <div className="space-y-4">
        {(node.kind.children ?? []).map((child) => (
          <NodeRenderer
            key={child.pointer}
            node={child}
            value={getPointerValue(data, child.pointer)}
            errors={errors}
            onChange={onChange}
          />
        ))}
      </div>
    );
  }
  return (
    <NodeRenderer
      node={node}
      value={getPointerValue(data, node.pointer)}
      errors={errors}
      onChange={onChange}
    />
  );
}

function pointerSegments(pointer?: string) {
  if (!pointer || pointer === '/') return [];
  return pointer
    .split('/')
    .filter(Boolean)
    .map((segment) => segment.replace(/~1/g, '/').replace(/~0/g, '~'));
}
