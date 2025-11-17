import type { SessionResponse } from "./types";

// Test data based on complex.schema.json
export const testData: SessionResponse = {
    title: "Test Complex Schema",
    blueprint: {
        title: "Complex Schema Test",
        description: "Testing anyOf/oneOf and deep nesting",
        roots: [],
    },
    formats: ["json"],
    data: {
        c: {
            c1: {
                c2: {
                    options: [], // anyOf: string[] | integer[]
                },
            },
        },
        e: {
            e1: {
                e2: {
                    e3: {
                        e4: {
                            deepItems: [], // array of anyOf
                            logic: { type: "fixed", value: 0 }, // oneOf
                        },
                    },
                },
            },
        },
        d: {
            d1: {
                d2: {
                    d3: {
                        config: {
                            features: [], // array of objects
                        },
                    },
                },
            },
        },
        b: {
            b1: [], // array of oneOf
        },
    },
    ui_ast: {
        roots: [
            {
                pointer: "/c",
                title: "C - Composite Tests",
                required: false,
                kind: {
                    type: "object",
                    children: [
                        {
                            pointer: "/c/c1",
                            title: "C1",
                            required: false,
                            kind: {
                                type: "object",
                                children: [
                                    {
                                        pointer: "/c/c1/c2",
                                        title: "C2",
                                        required: false,
                                        kind: {
                                            type: "object",
                                            children: [
                                                {
                                                    pointer: "/c/c1/c2/options",
                                                    title: "Options",
                                                    required: false,
                                                    kind: {
                                                        type: "composite",
                                                        mode: "any_of",
                                                        allow_multiple: false, // Should be single selection
                                                        variants: [
                                                            {
                                                                id: "variant_0",
                                                                title:
                                                                    "String Array",
                                                                is_object:
                                                                    false,
                                                                node: {
                                                                    type:
                                                                        "array",
                                                                    item: {
                                                                        type:
                                                                            "field",
                                                                        scalar:
                                                                            "string",
                                                                    },
                                                                },
                                                                schema: {
                                                                    type:
                                                                        "array",
                                                                    items: {
                                                                        type:
                                                                            "string",
                                                                    },
                                                                },
                                                            },
                                                            {
                                                                id: "variant_1",
                                                                title:
                                                                    "Integer Array",
                                                                is_object:
                                                                    false,
                                                                node: {
                                                                    type:
                                                                        "array",
                                                                    item: {
                                                                        type:
                                                                            "field",
                                                                        scalar:
                                                                            "integer",
                                                                    },
                                                                },
                                                                schema: {
                                                                    type:
                                                                        "array",
                                                                    items: {
                                                                        type:
                                                                            "integer",
                                                                    },
                                                                },
                                                            },
                                                        ],
                                                    },
                                                },
                                            ],
                                            required: [],
                                        },
                                    },
                                ],
                                required: [],
                            },
                        },
                    ],
                    required: [],
                },
            },
            {
                pointer: "/e",
                title: "E - Deep Nesting",
                required: false,
                kind: {
                    type: "object",
                    children: [
                        {
                            pointer: "/e/e1",
                            title: "E1",
                            required: false,
                            kind: {
                                type: "object",
                                children: [
                                    {
                                        pointer: "/e/e1/e2",
                                        title: "E2",
                                        required: false,
                                        kind: {
                                            type: "object",
                                            children: [
                                                {
                                                    pointer: "/e/e1/e2/e3",
                                                    title: "E3",
                                                    required: false,
                                                    kind: {
                                                        type: "object",
                                                        children: [
                                                            {
                                                                pointer:
                                                                    "/e/e1/e2/e3/e4",
                                                                title: "E4",
                                                                required: false,
                                                                kind: {
                                                                    type:
                                                                        "object",
                                                                    children: [
                                                                        {
                                                                            pointer:
                                                                                "/e/e1/e2/e3/e4/deepItems",
                                                                            title:
                                                                                "Deep Items",
                                                                            required:
                                                                                false,
                                                                            kind:
                                                                                {
                                                                                    type:
                                                                                        "array",
                                                                                    item:
                                                                                        {
                                                                                            type:
                                                                                                "composite",
                                                                                            mode:
                                                                                                "any_of",
                                                                                            allow_multiple:
                                                                                                false, // Each array item is single anyOf
                                                                                            variants:
                                                                                                [
                                                                                                    {
                                                                                                        id: "variant_0",
                                                                                                        title:
                                                                                                            "Target Object",
                                                                                                        is_object:
                                                                                                            true,
                                                                                                        node:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "object",
                                                                                                                children:
                                                                                                                    [
                                                                                                                        {
                                                                                                                            pointer:
                                                                                                                                "/url",
                                                                                                                            title:
                                                                                                                                "URL",
                                                                                                                            required:
                                                                                                                                true,
                                                                                                                            kind:
                                                                                                                                {
                                                                                                                                    type:
                                                                                                                                        "field",
                                                                                                                                    scalar:
                                                                                                                                        "string",
                                                                                                                                },
                                                                                                                        },
                                                                                                                        {
                                                                                                                            pointer:
                                                                                                                                "/priority",
                                                                                                                            title:
                                                                                                                                "Priority",
                                                                                                                            required:
                                                                                                                                false,
                                                                                                                            kind:
                                                                                                                                {
                                                                                                                                    type:
                                                                                                                                        "field",
                                                                                                                                    scalar:
                                                                                                                                        "integer",
                                                                                                                                },
                                                                                                                        },
                                                                                                                        {
                                                                                                                            pointer:
                                                                                                                                "/active",
                                                                                                                            title:
                                                                                                                                "Active",
                                                                                                                            required:
                                                                                                                                false,
                                                                                                                            kind:
                                                                                                                                {
                                                                                                                                    type:
                                                                                                                                        "field",
                                                                                                                                    scalar:
                                                                                                                                        "boolean",
                                                                                                                                },
                                                                                                                        },
                                                                                                                    ],
                                                                                                                required:
                                                                                                                    ["url"],
                                                                                                            },
                                                                                                        schema:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "object",
                                                                                                                properties:
                                                                                                                    {
                                                                                                                        url: {
                                                                                                                            type:
                                                                                                                                "string",
                                                                                                                        },
                                                                                                                        priority:
                                                                                                                            {
                                                                                                                                type:
                                                                                                                                    "integer",
                                                                                                                            },
                                                                                                                        active:
                                                                                                                            {
                                                                                                                                type:
                                                                                                                                    "boolean",
                                                                                                                            },
                                                                                                                    },
                                                                                                                required:
                                                                                                                    ["url"],
                                                                                                            },
                                                                                                    },
                                                                                                    {
                                                                                                        id: "variant_1",
                                                                                                        title:
                                                                                                            "String",
                                                                                                        is_object:
                                                                                                            false,
                                                                                                        node:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "field",
                                                                                                                scalar:
                                                                                                                    "string",
                                                                                                            },
                                                                                                        schema:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "string",
                                                                                                            },
                                                                                                    },
                                                                                                    {
                                                                                                        id: "variant_2",
                                                                                                        title:
                                                                                                            "Integer",
                                                                                                        is_object:
                                                                                                            false,
                                                                                                        node:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "field",
                                                                                                                scalar:
                                                                                                                    "integer",
                                                                                                            },
                                                                                                        schema:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "integer",
                                                                                                            },
                                                                                                    },
                                                                                                ],
                                                                                        },
                                                                                },
                                                                        },
                                                                        {
                                                                            pointer:
                                                                                "/e/e1/e2/e3/e4/logic",
                                                                            title:
                                                                                "Logic",
                                                                            required:
                                                                                false,
                                                                            kind:
                                                                                {
                                                                                    type:
                                                                                        "composite",
                                                                                    mode:
                                                                                        "one_of",
                                                                                    allow_multiple:
                                                                                        false,
                                                                                    variants:
                                                                                        [
                                                                                            {
                                                                                                id: "variant_0",
                                                                                                title:
                                                                                                    "Fixed Logic",
                                                                                                is_object:
                                                                                                    true,
                                                                                                node:
                                                                                                    {
                                                                                                        type:
                                                                                                            "object",
                                                                                                        children:
                                                                                                            [
                                                                                                                {
                                                                                                                    pointer:
                                                                                                                        "/type",
                                                                                                                    title:
                                                                                                                        "Type",
                                                                                                                    required:
                                                                                                                        true,
                                                                                                                    kind:
                                                                                                                        {
                                                                                                                            type:
                                                                                                                                "field",
                                                                                                                            scalar:
                                                                                                                                "string",
                                                                                                                        },
                                                                                                                },
                                                                                                                {
                                                                                                                    pointer:
                                                                                                                        "/value",
                                                                                                                    title:
                                                                                                                        "Value",
                                                                                                                    required:
                                                                                                                        true,
                                                                                                                    kind:
                                                                                                                        {
                                                                                                                            type:
                                                                                                                                "field",
                                                                                                                            scalar:
                                                                                                                                "number",
                                                                                                                        },
                                                                                                                },
                                                                                                            ],
                                                                                                        required:
                                                                                                            [
                                                                                                                "type",
                                                                                                                "value",
                                                                                                            ],
                                                                                                    },
                                                                                                schema:
                                                                                                    {
                                                                                                        type:
                                                                                                            "object",
                                                                                                        properties:
                                                                                                            {
                                                                                                                type:
                                                                                                                    {
                                                                                                                        type:
                                                                                                                            "string",
                                                                                                                        const:
                                                                                                                            "fixed",
                                                                                                                    },
                                                                                                                value:
                                                                                                                    {
                                                                                                                        type:
                                                                                                                            "number",
                                                                                                                    },
                                                                                                            },
                                                                                                        required:
                                                                                                            [
                                                                                                                "type",
                                                                                                                "value",
                                                                                                            ],
                                                                                                    },
                                                                                            },
                                                                                            {
                                                                                                id: "variant_1",
                                                                                                title:
                                                                                                    "Dynamic Logic",
                                                                                                is_object:
                                                                                                    true,
                                                                                                node:
                                                                                                    {
                                                                                                        type:
                                                                                                            "object",
                                                                                                        children:
                                                                                                            [
                                                                                                                {
                                                                                                                    pointer:
                                                                                                                        "/type",
                                                                                                                    title:
                                                                                                                        "Type",
                                                                                                                    required:
                                                                                                                        true,
                                                                                                                    kind:
                                                                                                                        {
                                                                                                                            type:
                                                                                                                                "field",
                                                                                                                            scalar:
                                                                                                                                "string",
                                                                                                                        },
                                                                                                                },
                                                                                                                {
                                                                                                                    pointer:
                                                                                                                        "/expression",
                                                                                                                    title:
                                                                                                                        "Expression",
                                                                                                                    required:
                                                                                                                        true,
                                                                                                                    kind:
                                                                                                                        {
                                                                                                                            type:
                                                                                                                                "field",
                                                                                                                            scalar:
                                                                                                                                "string",
                                                                                                                        },
                                                                                                                },
                                                                                                            ],
                                                                                                        required:
                                                                                                            [
                                                                                                                "type",
                                                                                                                "expression",
                                                                                                            ],
                                                                                                    },
                                                                                                schema:
                                                                                                    {
                                                                                                        type:
                                                                                                            "object",
                                                                                                        properties:
                                                                                                            {
                                                                                                                type:
                                                                                                                    {
                                                                                                                        type:
                                                                                                                            "string",
                                                                                                                        const:
                                                                                                                            "dynamic",
                                                                                                                    },
                                                                                                                expression:
                                                                                                                    {
                                                                                                                        type:
                                                                                                                            "string",
                                                                                                                    },
                                                                                                            },
                                                                                                        required:
                                                                                                            [
                                                                                                                "type",
                                                                                                                "expression",
                                                                                                            ],
                                                                                                    },
                                                                                            },
                                                                                        ],
                                                                                },
                                                                        },
                                                                    ],
                                                                    required:
                                                                        [],
                                                                },
                                                            },
                                                        ],
                                                        required: [],
                                                    },
                                                },
                                            ],
                                            required: [],
                                        },
                                    },
                                ],
                                required: [],
                            },
                        },
                    ],
                    required: [],
                },
            },
            {
                pointer: "/d",
                title: "D - Object Arrays",
                required: false,
                kind: {
                    type: "object",
                    children: [
                        {
                            pointer: "/d/d1",
                            title: "D1",
                            required: false,
                            kind: {
                                type: "object",
                                children: [
                                    {
                                        pointer: "/d/d1/d2",
                                        title: "D2",
                                        required: false,
                                        kind: {
                                            type: "object",
                                            children: [
                                                {
                                                    pointer: "/d/d1/d2/d3",
                                                    title: "D3",
                                                    required: false,
                                                    kind: {
                                                        type: "object",
                                                        children: [
                                                            {
                                                                pointer:
                                                                    "/d/d1/d2/d3/config",
                                                                title: "Config",
                                                                required: false,
                                                                kind: {
                                                                    type:
                                                                        "object",
                                                                    children: [
                                                                        {
                                                                            pointer:
                                                                                "/d/d1/d2/d3/config/features",
                                                                            title:
                                                                                "Features",
                                                                            required:
                                                                                false,
                                                                            kind:
                                                                                {
                                                                                    type:
                                                                                        "array",
                                                                                    item:
                                                                                        {
                                                                                            type:
                                                                                                "object",
                                                                                            children:
                                                                                                [
                                                                                                    {
                                                                                                        pointer:
                                                                                                            "/key",
                                                                                                        title:
                                                                                                            "Key",
                                                                                                        required:
                                                                                                            true,
                                                                                                        kind:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "field",
                                                                                                                scalar:
                                                                                                                    "string",
                                                                                                            },
                                                                                                    },
                                                                                                    {
                                                                                                        pointer:
                                                                                                            "/enabled",
                                                                                                        title:
                                                                                                            "Enabled",
                                                                                                        required:
                                                                                                            false,
                                                                                                        kind:
                                                                                                            {
                                                                                                                type:
                                                                                                                    "field",
                                                                                                                scalar:
                                                                                                                    "boolean",
                                                                                                            },
                                                                                                    },
                                                                                                ],
                                                                                            required:
                                                                                                ["key"],
                                                                                        },
                                                                                },
                                                                        },
                                                                    ],
                                                                    required:
                                                                        [],
                                                                },
                                                            },
                                                        ],
                                                        required: [],
                                                    },
                                                },
                                            ],
                                            required: [],
                                        },
                                    },
                                ],
                                required: [],
                            },
                        },
                    ],
                    required: [],
                },
            },
            {
                pointer: "/b",
                title: "B - Array of OneOf",
                required: false,
                kind: {
                    type: "object",
                    children: [
                        {
                            pointer: "/b/b1",
                            title: "B1",
                            required: false,
                            kind: {
                                type: "array",
                                item: {
                                    type: "composite",
                                    mode: "one_of",
                                    allow_multiple: false,
                                    variants: [
                                        {
                                            id: "variant_0",
                                            title: "Simple Item",
                                            is_object: true,
                                            node: {
                                                type: "object",
                                                children: [
                                                    {
                                                        pointer: "/type",
                                                        title: "Type",
                                                        required: true,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "string",
                                                        },
                                                    },
                                                    {
                                                        pointer: "/label",
                                                        title: "Label",
                                                        required: true,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "string",
                                                        },
                                                    },
                                                    {
                                                        pointer: "/active",
                                                        title: "Active",
                                                        required: false,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "boolean",
                                                        },
                                                    },
                                                ],
                                                required: ["type", "label"],
                                            },
                                            schema: {
                                                type: "object",
                                                properties: {
                                                    type: {
                                                        type: "string",
                                                        const: "simple",
                                                    },
                                                    label: { type: "string" },
                                                    active: { type: "boolean" },
                                                },
                                                required: ["type", "label"],
                                            },
                                        },
                                        {
                                            id: "variant_1",
                                            title: "Numeric Item",
                                            is_object: true,
                                            node: {
                                                type: "object",
                                                children: [
                                                    {
                                                        pointer: "/type",
                                                        title: "Type",
                                                        required: true,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "string",
                                                        },
                                                    },
                                                    {
                                                        pointer: "/count",
                                                        title: "Count",
                                                        required: true,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "integer",
                                                        },
                                                    },
                                                    {
                                                        pointer: "/threshold",
                                                        title: "Threshold",
                                                        required: false,
                                                        kind: {
                                                            type: "field",
                                                            scalar: "number",
                                                        },
                                                    },
                                                ],
                                                required: ["type", "count"],
                                            },
                                            schema: {
                                                type: "object",
                                                properties: {
                                                    type: {
                                                        type: "string",
                                                        const: "numeric",
                                                    },
                                                    count: { type: "integer" },
                                                    threshold: {
                                                        type: "number",
                                                    },
                                                },
                                                required: ["type", "count"],
                                            },
                                        },
                                    ],
                                },
                            },
                        },
                    ],
                    required: [],
                },
            },
        ],
    },
};
