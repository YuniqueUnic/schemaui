use std::{fs, path::PathBuf};

use serde_json::Value;

use crate::io::{DocumentFormat, input::parse_document_str};
use crate::precompile;
use crate::precompile::tui as ct_tui;
use crate::tui::model::form_schema_from_ui_ast;
use crate::tui::state::LayoutNavModel;
use crate::ui_ast::{build_ui_ast, index::collect_pointers, layout::build_ui_layout};

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schemas")
        .join("test-comprehensive.schema.json")
}

#[test]
fn precompile_and_runtime_ui_ast_match_for_comprehensive_schema() {
    let path = schema_path();

    let precompile_ast =
        precompile::build_ui_ast_from_file(&path, DocumentFormat::Json).expect("precompile UiAst");

    let contents = std::fs::read_to_string(&path).expect("schema file readable");
    let schema_value: Value =
        parse_document_str(&contents, DocumentFormat::Json).expect("schema parses at runtime");
    let runtime_ast = build_ui_ast(&schema_value).expect("runtime UiAst");

    let precompile_pointers = collect_pointers(&precompile_ast);
    let runtime_pointers = collect_pointers(&runtime_ast);

    assert_eq!(runtime_pointers, precompile_pointers);
}

#[test]
fn ui_ast_json_roundtrip_preserves_structure() {
    let path = schema_path();

    let original =
        precompile::build_ui_ast_from_file(&path, DocumentFormat::Json).expect("precompile UiAst");
    let json = precompile::ui_ast_to_json(&original).expect("UiAst to JSON");
    let decoded = precompile::decode_ui_ast(&json).expect("decode UiAst from JSON");

    assert_eq!(original, decoded);
}

#[test]
fn precompiled_form_schema_matches_runtime_form_schema() {
    let path = schema_path();

    // 1) Precompiled-style build via precompile::tui helper.
    let (ct_ast, ct_form) = ct_tui::build_tui_form_schema_from_file(&path, DocumentFormat::Json)
        .expect("precompile FormSchema");

    // 2) Runtime-style build from the same schema file.
    let contents = fs::read_to_string(&path).expect("schema file readable");
    let schema_value: Value =
        parse_document_str(&contents, DocumentFormat::Json).expect("schema parses at runtime");
    let rt_ast = build_ui_ast(&schema_value).expect("runtime UiAst");
    let rt_form = form_schema_from_ui_ast(&rt_ast);

    // 3) Assert UiAst and FormSchema equivalence (via JSON for FormSchema).
    if ct_ast != rt_ast {
        let ct_ptrs = collect_pointers(&ct_ast);
        let rt_ptrs = collect_pointers(&rt_ast);
        eprintln!(
            "UiAst mismatch: ct_pointers_len={} rt_pointers_len={}",
            ct_ptrs.len(),
            rt_ptrs.len()
        );
        let ct_sample: Vec<_> = ct_ptrs.iter().take(8).cloned().collect();
        let rt_sample: Vec<_> = rt_ptrs.iter().take(8).cloned().collect();
        eprintln!("  ct_pointers_sample={:?}", ct_sample);
        eprintln!("  rt_pointers_sample={:?}", rt_sample);

        // Persist full UiAst JSON to a temporary directory for manual diffing.
        let out_dir = std::env::var("CARGO_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(
                    std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()),
                )
                .join("tmp_precompiled_tui")
            });
        if let Err(err) = fs::create_dir_all(&out_dir) {
            eprintln!("  failed to create UiAst debug dir {:?}: {err}", out_dir);
        } else {
            let ct_json = serde_json::to_string_pretty(&ct_ast).unwrap();
            let rt_json = serde_json::to_string_pretty(&rt_ast).unwrap();
            let ct_path = out_dir.join("ct_ui_ast.json");
            let rt_path = out_dir.join("rt_ui_ast.json");
            if let Err(err) = fs::write(&ct_path, ct_json) {
                eprintln!("  failed to write {:?}: {err}", ct_path);
            }
            if let Err(err) = fs::write(&rt_path, rt_json) {
                eprintln!("  failed to write {:?}: {err}", rt_path);
            }
            eprintln!("  wrote UiAst JSON to {:?}", out_dir);
        }
    }
    assert_eq!(ct_ast, rt_ast, "precompiled and runtime UiAst must match");

    let ct_form_json = serde_json::to_value(&ct_form).expect("serialize ct FormSchema");
    let rt_form_json = serde_json::to_value(&rt_form).expect("serialize rt FormSchema");
    if ct_form_json != rt_form_json {
        eprintln!(
            "FormSchema JSON mismatch: ct_roots={} rt_roots={}",
            ct_form.roots.len(),
            rt_form.roots.len()
        );
        if let (Some(ct_root), Some(rt_root)) = (ct_form.roots.get(0), rt_form.roots.get(0)) {
            eprintln!(
                " root[0]: ct_id={} ct_sections={} rt_id={} rt_sections={}",
                ct_root.id,
                ct_root.sections.len(),
                rt_root.id,
                rt_root.sections.len()
            );
        }
    }
    assert_eq!(
        ct_form_json, rt_form_json,
        "precompiled and runtime FormSchema JSON must match"
    );
}

#[test]
fn precompiled_layout_nav_matches_runtime_layout_nav() {
    let path = schema_path();

    // 1) Precompiled-style build via precompile::tui helper.
    let (ct_ast, ct_nav): (_, LayoutNavModel) =
        ct_tui::build_tui_layout_nav_from_file(&path, DocumentFormat::Json)
            .expect("precompile LayoutNavModel");

    // 2) Runtime-style build from the same schema file.
    let contents = fs::read_to_string(&path).expect("schema file readable");
    let schema_value: Value =
        parse_document_str(&contents, DocumentFormat::Json).expect("schema parses at runtime");
    let rt_ast = build_ui_ast(&schema_value).expect("runtime UiAst");
    let rt_layout = build_ui_layout(&rt_ast);
    let rt_nav = LayoutNavModel::from_uilayout(&rt_layout);

    // 3) Assert UiAst and LayoutNavModel equivalence (via JSON for LayoutNavModel).
    if ct_ast != rt_ast {
        let ct_ptrs = collect_pointers(&ct_ast);
        let rt_ptrs = collect_pointers(&rt_ast);
        eprintln!(
            "UiAst mismatch: ct_pointers_len={} rt_pointers_len={}",
            ct_ptrs.len(),
            rt_ptrs.len()
        );
        let ct_sample: Vec<_> = ct_ptrs.iter().take(8).cloned().collect();
        let rt_sample: Vec<_> = rt_ptrs.iter().take(8).cloned().collect();
        eprintln!("  ct_pointers_sample={:?}", ct_sample);
        eprintln!("  rt_pointers_sample={:?}", rt_sample);
    }
    assert_eq!(ct_ast, rt_ast, "precompiled and runtime UiAst must match");

    let ct_nav_json = serde_json::to_value(&ct_nav).expect("serialize ct LayoutNavModel");
    let rt_nav_json = serde_json::to_value(&rt_nav).expect("serialize rt LayoutNavModel");
    if ct_nav_json != rt_nav_json {
        eprintln!(
            "LayoutNavModel JSON mismatch: ct_roots={} rt_roots={}",
            ct_nav.roots.len(),
            rt_nav.roots.len()
        );
        if let (Some(ct_root), Some(rt_root)) = (ct_nav.roots.get(0), rt_nav.roots.get(0)) {
            eprintln!(
                " root[0]: ct_id={} ct_sections={} rt_id={} rt_sections={}",
                ct_root.id,
                ct_root.sections.len(),
                rt_root.id,
                rt_root.sections.len()
            );
            if let (Some(ct_section), Some(rt_section)) =
                (ct_root.sections.get(0), rt_root.sections.get(0))
            {
                eprintln!(
                    " section[0]: ct_id={} ct_pointers_len={} rt_id={} rt_pointers_len={}",
                    ct_section.id,
                    ct_section.pointers.len(),
                    rt_section.id,
                    rt_section.pointers.len()
                );
                let ct_sample: Vec<_> = ct_section.pointers.iter().take(8).cloned().collect();
                let rt_sample: Vec<_> = rt_section.pointers.iter().take(8).cloned().collect();
                eprintln!("  ct_section_pointers_sample={:?}", ct_sample);
                eprintln!("  rt_section_pointers_sample={:?}", rt_sample);
            }
        }
    }
    assert_eq!(
        ct_nav_json, rt_nav_json,
        "precompiled and runtime LayoutNavModel JSON must match"
    );
}

#[test]
fn generated_tui_modules_produce_expected_artifacts() {
    let path = schema_path();

    // 1) Generate modules into a temporary directory under target/.
    let out_dir = std::env::var("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(
                std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()),
            )
            .join("tmp_precompiled_tui")
        });
    fs::create_dir_all(&out_dir).expect("tmp dir creatable");

    let form_module = out_dir.join("precompiled_form_schema.rs");
    let nav_module = out_dir.join("precompiled_layout_nav.rs");

    ct_tui::generate_tui_form_schema_module(
        &path,
        DocumentFormat::Json,
        &form_module,
        "precompiled_form_schema",
    )
    .expect("generate FormSchema module");
    ct_tui::generate_tui_layout_nav_module(
        &path,
        DocumentFormat::Json,
        &nav_module,
        "precompiled_layout_nav",
    )
    .expect("generate LayoutNavModel module");

    // 2) Sanity-check that the generated files exist and contain the expected
    // function names. We do not compile/include them here to keep this test
    // fast and independent from include! paths.
    let form_src = fs::read_to_string(&form_module).expect("form module readable");
    assert!(form_src.contains("fn precompiled_form_schema"));

    let nav_src = fs::read_to_string(&nav_module).expect("nav module readable");
    assert!(nav_src.contains("fn precompiled_layout_nav"));
}
