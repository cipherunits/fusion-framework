//! Tera template rendering with built-in Fusion UI component macros.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde_json::Value;
use tera::{Context, Tera};

const BUILTIN_MACROS: &str = include_str!("../assets/templates/fusion/macros.html");
const BUILTIN_BASE: &str = include_str!("../assets/templates/fusion/base.html");
const BUILTIN_COMPONENTS_CSS: &str =
    include_str!("../assets/templates/fusion/components.css");

static ENGINE_CACHE: Mutex<Option<EngineCache>> = Mutex::new(None);

struct EngineCache {
    key: String,
    tera: Tera,
}

/// Render a template file (path relative to the templates root) with a JSON context.
pub fn render_template(
    template_name: &str,
    context: &Value,
    templates_root: &Path,
) -> Result<String, String> {
    let tera = engine_for_root(templates_root)?;
    let ctx =
        Context::from_serialize(context).map_err(|e| format!("invalid template context: {e}"))?;
    tera.render(template_name, &ctx)
        .map_err(|e| format!("template render failed: {e}"))
}

fn engine_for_root(root: &Path) -> Result<Tera, String> {
    let key = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .to_string();

    let mut guard = ENGINE_CACHE
        .lock()
        .map_err(|_| "template engine lock poisoned".to_string())?;
    if let Some(cache) = guard.as_ref() {
        if cache.key == key {
            return Ok(cache.tera.clone());
        }
    }

    let tera = build_engine(root)?;
    *guard = Some(EngineCache {
        key,
        tera: tera.clone(),
    });
    Ok(tera)
}

fn build_engine(root: &Path) -> Result<Tera, String> {
    let mut raw: Vec<(String, String)> = vec![
        ("fusion/macros.html".to_string(), BUILTIN_MACROS.to_string()),
        ("fusion/base.html".to_string(), BUILTIN_BASE.to_string()),
        (
            "fusion/components.css".to_string(),
            BUILTIN_COMPONENTS_CSS.to_string(),
        ),
    ];

    if root.is_dir() {
        collect_templates(root, root, &mut raw)?;
    }

    let pairs: Vec<(&str, &str)> = raw.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    let mut tera = Tera::default();
    tera.add_raw_templates(pairs)
        .map_err(|e| format!("failed to load templates: {e}"))?;
    Ok(tera)
}

fn collect_templates(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, String)>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_templates(root, &path, out)?;
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "html" && ext != "tera" && ext != "css" {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        out.push((rel, content));
    }
    Ok(())
}

/// Clear cached Tera engine (useful in tests or hot-reload).
pub fn clear_template_cache() {
    if let Ok(mut guard) = ENGINE_CACHE.lock() {
        *guard = None;
    }
}

/// List built-in component names exposed to templates.
pub fn builtin_components() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (
            "button",
            "{{<fusion.button label=\"...\" href=\"...\" variant=\"primary\" />}}",
        ),
        ("link", "{{<fusion.link label=\"...\" href=\"...\" />}}"),
        ("card", "{{<fusion.card title=\"...\" content=\"...\" />}}"),
        (
            "alert",
            "{{<fusion.alert message={msg} variant=\"info\" />}}",
        ),
        (
            "badge",
            "{{<fusion.badge label=\"...\" variant=\"success\" dot={true} />}}",
        ),
        (
            "table",
            "{{<fusion.table headers={cols} rows={rows} caption=\"...\" />}}",
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_builtin_macro() {
        clear_template_cache();
        let tpl = r#"{{<fusion.button label="Go" href="/" />}}"#;
        let dir = std::env::temp_dir().join("fusion_tpl_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), tpl).unwrap();
        let html = render_template("test.html", &json!({}), &dir).unwrap();
        assert!(html.contains("fusion-btn"));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("Go"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn renders_badge_with_dot() {
        clear_template_cache();
        let tpl =
            r#"{{<fusion.badge label="Installation successful" variant="success" dot={true} />}}"#;
        let dir = std::env::temp_dir().join("fusion_tpl_badge_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), tpl).unwrap();
        let html = render_template("test.html", &json!({}), &dir).unwrap();
        assert!(html.contains("fusion-badge--success"));
        assert!(html.contains("fusion-badge__dot"));
        assert!(html.contains("Installation successful"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn renders_table_from_arrays() {
        clear_template_cache();
        let tpl = r#"{{<fusion.table headers={headers} rows={rows} caption="Products" />}}"#;
        let dir = std::env::temp_dir().join("fusion_tpl_table_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), tpl).unwrap();
        let html = render_template(
            "test.html",
            &json!({
                "headers": ["Name", "Status"],
                "rows": [["Widget", "ok"], ["Gadget", "draft"]],
            }),
            &dir,
        )
        .unwrap();
        assert!(html.contains("fusion-table"));
        assert!(html.contains("<th scope=\"col\">Name</th>"));
        assert!(html.contains("<td>Widget</td>"));
        assert!(html.contains("Products"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn includes_css_partial() {
        clear_template_cache();
        let dir = std::env::temp_dir().join("fusion_tpl_css_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("home")).unwrap();
        std::fs::write(dir.join("home/style.css"), "body { color: red; }").unwrap();
        std::fs::write(
            dir.join("home/index.html"),
            r#"<style>{% include "home/style.css" %}</style>"#,
        )
        .unwrap();
        let html = render_template("home/index.html", &json!({}), &dir).unwrap();
        assert!(html.contains("color: red"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn renders_card_with_body_slot() {
        clear_template_cache();
        let tpl = r#"{% <fusion.card title="Get started"> %}<div class="code">hello</div>{% </fusion.card> %}"#;
        let dir = std::env::temp_dir().join("fusion_tpl_card_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), tpl).unwrap();
        let html = render_template("test.html", &json!({}), &dir).unwrap();
        assert!(html.contains("fusion-card"));
        assert!(html.contains("Get started"));
        assert!(html.contains("hello"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn includes_builtin_components_css() {
        clear_template_cache();
        let dir = std::env::temp_dir().join("fusion_tpl_components_css_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("test.html"),
            r#"<style>{% include "fusion/components.css" %}</style>"#,
        )
        .unwrap();
        let html = render_template("test.html", &json!({}), &dir).unwrap();
        assert!(html.contains(".fusion-btn"));
        assert!(html.contains(".fusion-table"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
