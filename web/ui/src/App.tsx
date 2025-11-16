import { useEffect, useRef, useState } from 'react';
import { AppHeader } from './components/AppHeader';
import { NodeRenderer } from './components/NodeRenderer';
import { PreviewPane } from './components/PreviewPane';
import { StatusBar } from './components/StatusBar';
import { OverlayProvider } from './components/Overlay';
import {
  exitSession,
  fetchSession,
  persistData,
  renderPreview,
  validateData,
} from './api';
import type { JsonValue, SessionResponse } from './types';
import { applyUiDefaults } from './ui-ast';
import { deepClone, getPointerValue, setPointerValue } from './utils/jsonPointer';
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
  const validationSeq = useRef(0);
  const previewSeq = useRef(0);

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const payload = await fetchSession();
        if (!mounted) return;
        const withDefaults = applyUiDefaults(payload.ui_ast, payload.data ?? {});
        setSession(payload);
        setData(withDefaults);
        const availableFormats =
          payload.formats?.length && payload.formats.length > 0 ? payload.formats : ['json'];
        setFormats(availableFormats);
        setPreviewFormat(availableFormats.includes('json') ? 'json' : availableFormats[0]);
        setStatus('Ready');
        await Promise.all([runValidation(withDefaults), updatePreview(withDefaults, previewPretty, availableFormats[0])]);
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
    setErrors(new Map(errors)); // trigger state update
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

  const headerTitle = session?.title;

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center text-slate-600 dark:text-slate-300">
        Loading session…
      </div>
    );
  }

  return (
    <OverlayProvider>
      <div className="flex h-screen flex-col bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
        <AppHeader
          title={headerTitle}
          description={undefined}
          dirty={dirty}
          saving={saving}
          exiting={exiting}
          onSave={handleSave}
          onExit={handleExit}
        />
        <main className="flex min-h-0 flex-1 divide-x divide-slate-200 dark:divide-slate-800">
          <section className="min-h-0 flex-1 overflow-auto p-6">
            <div className="mx-auto flex max-w-5xl flex-col gap-4">
              {session?.ui_ast?.roots?.map((node) => (
                <NodeRenderer
                  key={node.pointer}
                  node={node}
                  value={getPointerValue(data, node.pointer)}
                  errors={errors}
                  onChange={handleChange}
                />
              ))}
            </div>
          </section>
          <section className="w-[38%] min-w-[320px]">
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
        </main>
        <StatusBar
          status={status}
          dirty={dirty}
          validating={false}
          errorCount={errors.size}
        />
      </div>
    </OverlayProvider>
  );
}
