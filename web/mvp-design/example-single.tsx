import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  AlertCircle,
  CheckCircle,
  ChevronDown,
  ChevronRight,
  FileCode,
  FileJson,
  Menu,
  Save,
  Search,
  X,
} from "lucide-react";

// ============================================================================
// 模拟后端通信层
// ============================================================================
class SchemaUIBackend {
  constructor() {
    this.schema = null;
    this.data = {};
  }

  async loadSchema(schema) {
    this.schema = schema;
    return { success: true };
  }

  async validate(data) {
    // 简化的验证逻辑 - 实际会调用 Rust 的 jsonschema 验证
    await new Promise((resolve) => setTimeout(resolve, 50));
    const errors = [];

    if (this.schema && this.schema.required) {
      this.schema.required.forEach((field) => {
        if (!data[field]) {
          errors.push({
            path: `/${field}`,
            message: `Field '${field}' is required`,
          });
        }
      });
    }

    return { valid: errors.length === 0, errors };
  }

  async save(data) {
    await new Promise((resolve) => setTimeout(resolve, 100));
    this.data = data;
    return { success: true, data };
  }
}

// ============================================================================
// 语法高亮组件（完全离线）
// ============================================================================
const SyntaxHighlight = ({ code, language }) => {
  const highlightJSON = (text) => {
    return text
      .replace(/("[\w\d_-]+")\s*:/g, '<span class="text-cyan-400">$1</span>:')
      .replace(/:\s*(".*?")/g, ': <span class="text-green-400">$1</span>')
      .replace(/:\s*(\d+\.?\d*)/g, ': <span class="text-orange-400">$1</span>')
      .replace(
        /:\s*(true|false|null)/g,
        ': <span class="text-purple-400">$1</span>',
      );
  };

  const highlightYAML = (text) => {
    return text
      .replace(/^(\s*[\w\d_-]+):/gm, '<span class="text-cyan-400">$1</span>:')
      .replace(/:\s*(".*?")/g, ': <span class="text-green-400">$1</span>')
      .replace(/:\s*(\d+\.?\d*)/g, ': <span class="text-orange-400">$1</span>')
      .replace(
        /:\s*(true|false|null)/g,
        ': <span class="text-purple-400">$1</span>',
      );
  };

  const highlightTOML = (text) => {
    return text
      .replace(
        /^\[.*\]/gm,
        (match) => `<span class="text-purple-400">${match}</span>`,
      )
      .replace(/^(\w+)\s*=/gm, '<span class="text-cyan-400">$1</span> =')
      .replace(
        /=\s*".*?"/g,
        (match) => `= <span class="text-green-400">${match.slice(2)}</span>`,
      )
      .replace(
        /=\s*\d+\.?\d*/g,
        (match) => `= <span class="text-orange-400">${match.slice(2)}</span>`,
      )
      .replace(
        /=\s*(true|false)/g,
        (match) => `= <span class="text-purple-400">${match.slice(2)}</span>`,
      );
  };

  const highlighted = useMemo(() => {
    let result = code;
    switch (language) {
      case "json":
        result = highlightJSON(code);
        break;
      case "yaml":
        result = highlightYAML(code);
        break;
      case "toml":
        result = highlightTOML(code);
        break;
      default:
        result = code;
    }
    return result;
  }, [code, language]);

  return (
    <pre className="text-sm leading-relaxed overflow-auto h-full p-4 bg-slate-900/50 rounded-lg">
      <code dangerouslySetInnerHTML={{ __html: highlighted }} />
    </pre>
  );
};

// ============================================================================
// 树节点组件
// ============================================================================
const TreeNode = (
  { node, level = 0, selectedPath, onSelect, expandedNodes, onToggle },
) => {
  const isExpanded = expandedNodes.has(node.path);
  const isSelected = selectedPath === node.path;
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div>
      <div
        className={`
          flex items-center gap-2 py-2 px-3 cursor-pointer rounded-lg
          transition-all duration-200 hover:bg-slate-700/50
          ${isSelected ? "bg-cyan-500/20 border-l-2 border-cyan-400" : ""}
        `}
        style={{ paddingLeft: `${level * 16 + 12}px` }}
        onClick={() => onSelect(node.path)}
      >
        {hasChildren && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node.path);
            }}
            className="p-1 hover:bg-slate-600/50 rounded transition-colors"
          >
            {isExpanded
              ? <ChevronDown size={16} className="text-slate-400" />
              : <ChevronRight size={16} className="text-slate-400" />}
          </button>
        )}
        {!hasChildren && <div className="w-6" />}
        <span
          className={`text-sm ${
            isSelected ? "text-cyan-300 font-medium" : "text-slate-300"
          }`}
        >
          {node.title}
        </span>
        {node.required && <span className="text-xs text-red-400">*</span>}
      </div>
      {isExpanded && hasChildren && (
        <div>
          {node.children.map((child, idx) => (
            <TreeNode
              key={idx}
              node={child}
              level={level + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              expandedNodes={expandedNodes}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
};

// ============================================================================
// 字段编辑器组件
// ============================================================================
const FieldEditor = ({ field, value, onChange, error }) => {
  const inputRef = useRef(null);

  const renderInput = () => {
    const baseClasses = `
      w-full px-4 py-2.5 bg-slate-800/50 border rounded-lg
      text-slate-200 placeholder-slate-500
      focus:outline-none focus:ring-2 focus:ring-cyan-500/50
      transition-all duration-200
      ${error ? "border-red-500/50" : "border-slate-600/50"}
    `;

    switch (field.type) {
      case "string":
        if (field.enum) {
          return (
            <select
              value={value || ""}
              onChange={(e) => onChange(e.target.value)}
              className={baseClasses}
            >
              <option value="">Select...</option>
              {field.enum.map((opt) => (
                <option key={opt} value={opt}>{opt}</option>
              ))}
            </select>
          );
        }
        return (
          <input
            ref={inputRef}
            type="text"
            value={value || ""}
            onChange={(e) => onChange(e.target.value)}
            placeholder={field.description || `Enter ${field.title}`}
            className={baseClasses}
          />
        );

      case "number":
      case "integer":
        return (
          <input
            ref={inputRef}
            type="number"
            value={value || ""}
            onChange={(e) => onChange(e.target.valueAsNumber)}
            placeholder={field.description || `Enter ${field.title}`}
            min={field.minimum}
            max={field.maximum}
            className={baseClasses}
          />
        );

      case "boolean":
        return (
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={!!value}
              onChange={(e) => onChange(e.target.checked)}
              className="w-5 h-5 rounded bg-slate-800 border-slate-600 text-cyan-500 
                         focus:ring-2 focus:ring-cyan-500/50 cursor-pointer"
            />
            <span className="text-slate-300">
              {field.description || "Enable"}
            </span>
          </label>
        );

      default:
        return (
          <textarea
            ref={inputRef}
            value={value || ""}
            onChange={(e) => onChange(e.target.value)}
            placeholder={field.description || `Enter ${field.title}`}
            rows={4}
            className={baseClasses}
          />
        );
    }
  };

  return (
    <div className="space-y-2">
      <label className="flex items-center gap-2 text-sm font-medium text-slate-300">
        {field.title}
        {field.required && <span className="text-red-400">*</span>}
      </label>
      {field.description && (
        <p className="text-xs text-slate-500">{field.description}</p>
      )}
      {renderInput()}
      {error && (
        <div className="flex items-center gap-2 text-xs text-red-400 bg-red-500/10 px-3 py-2 rounded-lg">
          <AlertCircle size={14} />
          <span>{error}</span>
        </div>
      )}
    </div>
  );
};

// ============================================================================
// 主应用组件
// ============================================================================
const SchemaUIWeb = () => {
  // 示例 Schema
  const demoSchema = {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "Application Configuration",
    "type": "object",
    "required": ["metadata", "server"],
    "properties": {
      "metadata": {
        "type": "object",
        "title": "Metadata",
        "required": ["name", "version"],
        "properties": {
          "name": {
            "type": "string",
            "title": "Service Name",
            "description": "The name of the service",
          },
          "version": {
            "type": "string",
            "title": "Version",
            "description": "Semantic version",
          },
          "environment": {
            "type": "string",
            "title": "Environment",
            "enum": ["dev", "staging", "prod"],
            "description": "Deployment environment",
          },
        },
      },
      "server": {
        "type": "object",
        "title": "Server Configuration",
        "properties": {
          "host": {
            "type": "string",
            "title": "Host",
            "default": "0.0.0.0",
          },
          "port": {
            "type": "integer",
            "title": "Port",
            "minimum": 1024,
            "maximum": 65535,
            "default": 8080,
          },
          "tls": {
            "type": "object",
            "title": "TLS Settings",
            "properties": {
              "enabled": {
                "type": "boolean",
                "title": "Enable TLS",
              },
              "cert_path": {
                "type": "string",
                "title": "Certificate Path",
              },
            },
          },
        },
      },
    },
  };

  const [backend] = useState(() => new SchemaUIBackend());
  const [schema, setSchema] = useState(demoSchema);
  const [formData, setFormData] = useState({});
  const [selectedPath, setSelectedPath] = useState("/metadata");
  const [expandedNodes, setExpandedNodes] = useState(
    new Set(["/metadata", "/server"]),
  );
  const [validationErrors, setValidationErrors] = useState([]);
  const [previewFormat, setPreviewFormat] = useState("json");
  const [saveStatus, setSaveStatus] = useState(null);
  const [isDirty, setIsDirty] = useState(false);

  // 构建树结构
  const treeData = useMemo(() => {
    const buildTree = (schema, path = "") => {
      if (!schema.properties) return [];

      return Object.entries(schema.properties).map(([key, prop]) => {
        const nodePath = path ? `${path}/${key}` : `/${key}`;
        const node = {
          path: nodePath,
          title: prop.title || key,
          type: prop.type,
          required: schema.required?.includes(key),
          children: [],
        };

        if (prop.type === "object" && prop.properties) {
          node.children = buildTree(prop, nodePath);
        }

        return node;
      });
    };

    return buildTree(schema);
  }, [schema]);

  // 获取当前选中节点的字段
  const currentFields = useMemo(() => {
    const getFieldsAtPath = (schema, path) => {
      if (path === "/") return [];

      const parts = path.split("/").filter(Boolean);
      let current = schema;

      for (const part of parts) {
        if (!current.properties || !current.properties[part]) return [];
        current = current.properties[part];
      }

      if (!current.properties) return [];

      return Object.entries(current.properties).map(([key, prop]) => ({
        key,
        title: prop.title || key,
        type: prop.type,
        description: prop.description,
        required: current.required?.includes(key),
        enum: prop.enum,
        minimum: prop.minimum,
        maximum: prop.maximum,
      }));
    };

    return getFieldsAtPath(schema, selectedPath);
  }, [schema, selectedPath]);

  // 获取/设置字段值
  const getFieldValue = useCallback((fieldKey) => {
    const path = selectedPath.split("/").filter(Boolean);
    let current = formData;

    for (const part of path) {
      if (!current[part]) return undefined;
      current = current[part];
    }

    return current[fieldKey];
  }, [formData, selectedPath]);

  const setFieldValue = useCallback((fieldKey, value) => {
    setIsDirty(true);
    const path = selectedPath.split("/").filter(Boolean);
    const newData = { ...formData };

    let current = newData;
    for (let i = 0; i < path.length; i++) {
      if (!current[path[i]]) {
        current[path[i]] = {};
      }
      if (i === path.length - 1) {
        if (!current[path[i]]) current[path[i]] = {};
        current[path[i]][fieldKey] = value;
      } else {
        current = current[path[i]];
      }
    }

    if (path.length === 0) {
      newData[fieldKey] = value;
    }

    setFormData(newData);
  }, [formData, selectedPath]);

  // 实时验证（去抖）
  useEffect(() => {
    const timer = setTimeout(async () => {
      const result = await backend.validate(formData);
      setValidationErrors(result.errors);
    }, 300);

    return () => clearTimeout(timer);
  }, [formData, backend]);

  // 格式化输出
  const formattedOutput = useMemo(() => {
    try {
      switch (previewFormat) {
        case "json":
          return JSON.stringify(formData, null, 2);
        case "yaml":
          // 简化的 YAML 输出
          const yamlify = (obj, indent = 0) => {
            const spaces = "  ".repeat(indent);
            return Object.entries(obj).map(([k, v]) => {
              if (typeof v === "object" && v !== null && !Array.isArray(v)) {
                return `${spaces}${k}:\n${yamlify(v, indent + 1)}`;
              }
              return `${spaces}${k}: ${JSON.stringify(v)}`;
            }).join("\n");
          };
          return yamlify(formData);
        case "toml":
          // 简化的 TOML 输出
          const tomlify = (obj, section = "") => {
            let result = [];
            for (const [k, v] of Object.entries(obj)) {
              if (typeof v === "object" && v !== null && !Array.isArray(v)) {
                const newSection = section ? `${section}.${k}` : k;
                result.push(`\n[${newSection}]`);
                result.push(tomlify(v, newSection));
              } else {
                result.push(`${k} = ${JSON.stringify(v)}`);
              }
            }
            return result.join("\n");
          };
          return tomlify(formData);
        default:
          return JSON.stringify(formData, null, 2);
      }
    } catch (e) {
      return `Error formatting: ${e.message}`;
    }
  }, [formData, previewFormat]);

  // 保存处理
  const handleSave = async () => {
    const result = await backend.save(formData);
    if (result.success) {
      setSaveStatus("success");
      setIsDirty(false);
      setTimeout(() => setSaveStatus(null), 2000);
    }
  };

  // 退出处理
  const handleExit = () => {
    if (isDirty && !confirm("You have unsaved changes. Exit anyway?")) {
      return;
    }
    console.log("Final output:", JSON.stringify(formData, null, 2));
    alert(
      "In real implementation, this would close the server and output to terminal",
    );
  };

  const toggleNode = (path) => {
    setExpandedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  return (
    <div className="h-screen flex flex-col bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-slate-100">
      {/* 顶部工具栏 */}
      <div className="h-14 bg-slate-800/80 backdrop-blur-lg border-b border-slate-700/50 flex items-center justify-between px-6 shadow-lg">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold bg-gradient-to-r from-cyan-400 to-blue-500 bg-clip-text text-transparent">
            SchemaUI Web
          </h1>
          {isDirty && (
            <span className="text-xs text-amber-400 flex items-center gap-1">
              <div className="w-2 h-2 bg-amber-400 rounded-full animate-pulse" />
              Unsaved changes
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {validationErrors.length > 0 && (
            <div className="flex items-center gap-2 text-sm text-red-400 bg-red-500/10 px-3 py-1.5 rounded-lg">
              <AlertCircle size={16} />
              <span>{validationErrors.length} error(s)</span>
            </div>
          )}

          {saveStatus === "success" && (
            <div className="flex items-center gap-2 text-sm text-green-400 bg-green-500/10 px-3 py-1.5 rounded-lg">
              <CheckCircle size={16} />
              <span>Saved!</span>
            </div>
          )}

          <button
            onClick={handleSave}
            disabled={validationErrors.length > 0}
            className="flex items-center gap-2 px-4 py-2 bg-cyan-500 hover:bg-cyan-600 
                     disabled:bg-slate-700 disabled:text-slate-500 disabled:cursor-not-allowed
                     text-white rounded-lg transition-colors shadow-lg shadow-cyan-500/20"
          >
            <Save size={18} />
            <span className="font-medium">Save</span>
          </button>

          <button
            onClick={handleExit}
            className="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 
                     text-slate-200 rounded-lg transition-colors"
          >
            <X size={18} />
            <span className="font-medium">Exit</span>
          </button>
        </div>
      </div>

      {/* 主内容区 - 三栏布局 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 左侧：树状导航 */}
        <div className="w-72 bg-slate-800/40 backdrop-blur-sm border-r border-slate-700/50 overflow-y-auto">
          <div className="p-4 border-b border-slate-700/50">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">
              Schema Navigator
            </h2>
          </div>
          <div className="p-2">
            {treeData.map((node, idx) => (
              <TreeNode
                key={idx}
                node={node}
                selectedPath={selectedPath}
                onSelect={setSelectedPath}
                expandedNodes={expandedNodes}
                onToggle={toggleNode}
              />
            ))}
          </div>
        </div>

        {/* 中间：编辑器 */}
        <div className="flex-1 overflow-y-auto bg-slate-900/30">
          <div className="p-6 space-y-6">
            <div className="bg-slate-800/40 backdrop-blur-sm rounded-xl p-6 border border-slate-700/30 shadow-xl">
              <h2 className="text-lg font-semibold text-slate-200 mb-6 flex items-center gap-2">
                <FileCode size={20} className="text-cyan-400" />
                {selectedPath.split("/").filter(Boolean).pop() || "Root"}
              </h2>

              <div className="space-y-6">
                {currentFields.length === 0
                  ? (
                    <div className="text-center py-12 text-slate-500">
                      <Menu size={48} className="mx-auto mb-4 opacity-30" />
                      <p>Select a section to edit fields</p>
                    </div>
                  )
                  : (
                    currentFields.map((field) => (
                      <FieldEditor
                        key={field.key}
                        field={field}
                        value={getFieldValue(field.key)}
                        onChange={(value) => setFieldValue(field.key, value)}
                        error={validationErrors.find((e) =>
                          e.path === `${selectedPath}/${field.key}`
                        )?.message}
                      />
                    ))
                  )}
              </div>
            </div>
          </div>
        </div>

        {/* 右侧：预览 */}
        <div className="w-96 bg-slate-800/40 backdrop-blur-sm border-l border-slate-700/50 flex flex-col">
          <div className="p-4 border-b border-slate-700/50 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">
              Preview
            </h2>
            <div className="flex gap-2">
              {["json", "yaml", "toml"].map((fmt) => (
                <button
                  key={fmt}
                  onClick={() => setPreviewFormat(fmt)}
                  className={`
                    px-3 py-1 text-xs rounded-lg transition-all uppercase font-medium
                    ${
                    previewFormat === fmt
                      ? "bg-cyan-500 text-white shadow-lg shadow-cyan-500/30"
                      : "bg-slate-700/50 text-slate-400 hover:bg-slate-700"
                  }
                  `}
                >
                  {fmt}
                </button>
              ))}
            </div>
          </div>

          <div className="flex-1 overflow-hidden p-4">
            <SyntaxHighlight code={formattedOutput} language={previewFormat} />
          </div>
        </div>
      </div>
    </div>
  );
};

export default SchemaUIWeb;
