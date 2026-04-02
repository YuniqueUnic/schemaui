import type { SessionResponse } from '@schemaui/types/SessionResponse';

export const PrecompiledSession: SessionResponse = {
  "title": null,
  "description": null,
  "ui_ast": {
    "roots": [
      {
        "pointer": "/age",
        "title": null,
        "description": "年龄",
        "required": true,
        "default_value": 0,
        "kind": {
          "type": "field",
          "scalar": "integer",
          "enum_options": null
        }
      },
      {
        "pointer": "/email",
        "title": null,
        "description": "电子邮箱地址",
        "required": true,
        "default_value": "",
        "kind": {
          "type": "field",
          "scalar": "string",
          "enum_options": null
        }
      },
      {
        "pointer": "/phone",
        "title": null,
        "description": "手机号（可选）",
        "required": false,
        "default_value": "",
        "kind": {
          "type": "field",
          "scalar": "string",
          "enum_options": null
        }
      },
      {
        "pointer": "/tags",
        "title": null,
        "description": "用户标签（可选）",
        "required": false,
        "default_value": [],
        "kind": {
          "type": "array",
          "item": {
            "type": "field",
            "scalar": "string",
            "enum_options": null
          },
          "min_items": 1,
          "max_items": 10
        }
      },
      {
        "pointer": "/username",
        "title": null,
        "description": "用户名，只允许字母、数字和下划线",
        "required": true,
        "default_value": "unic@me",
        "kind": {
          "type": "field",
          "scalar": "string",
          "enum_options": null
        }
      },
      {
        "pointer": "/website",
        "title": null,
        "description": "个人网站（可选）",
        "required": false,
        "default_value": "https://www.yunique.top",
        "kind": {
          "type": "field",
          "scalar": "string",
          "enum_options": null
        }
      }
    ]
  },
  "data": {},
  "formats": [
    "json",
    "yaml",
    "toml"
  ],
  "layout": {
    "roots": [
      {
        "id": "general",
        "title": "General",
        "description": null,
        "sections": [
          {
            "id": "general",
            "title": "General",
            "description": null,
            "pointer": "",
            "path": [],
            "field_pointers": [
              "/age",
              "/email",
              "/phone",
              "/tags",
              "/username",
              "/website"
            ],
            "children": []
          }
        ]
      }
    ]
  }
} as const;
