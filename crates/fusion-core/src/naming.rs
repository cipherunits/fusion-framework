//! Shared route naming helpers used by all language bindings.

/// Canonical HTTP handler method names (lowercase).
pub const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

/// `MyFirstModule` → `myfirst`; names without a `Module`/`MODULE` suffix are lowercased as-is.
pub fn api_resource_name(class_name: &str) -> String {
    let stem = if let Some(stem) = class_name.strip_suffix("Module") {
        if !stem.is_empty() {
            stem
        } else {
            class_name
        }
    } else if let Some(stem) = class_name.strip_suffix("MODULE") {
        if !stem.is_empty() {
            stem
        } else {
            class_name
        }
    } else {
        class_name
    };
    stem.to_lowercase()
}

/// `UserAction` → `user`; strips trailing `Action` / `ACTION` then lowercases.
pub fn api_action_name(method_name: &str) -> String {
    let stem = if let Some(stem) = method_name.strip_suffix("Action") {
        if !stem.is_empty() {
            stem
        } else {
            method_name
        }
    } else if let Some(stem) = method_name.strip_suffix("ACTION") {
        if !stem.is_empty() {
            stem
        } else {
            method_name
        }
    } else {
        method_name
    };
    stem.to_lowercase()
}

/// Expand the reserved `[module]` token using the module class name.
///
/// Example: `/api/[module]/{id}` + `ProductsModule` → `/api/products/{id}`.
/// Other `{param}` / `[param]` segments are left for the router.
pub fn resolve_route_path(template: &str, class_name: &str) -> String {
    template.replace("[module]", &api_resource_name(class_name))
}

/// Expand `[module]` and `[action]` tokens for a handler method route template.
pub fn resolve_method_route_path(template: &str, class_name: &str, method_name: &str) -> String {
    resolve_route_path(template, class_name).replace("[action]", &api_action_name(method_name))
}

/// Join a class-level base path with a method route segment.
pub fn join_route_paths(base: &str, segment: &str) -> String {
    let base = base.trim_end_matches('/');
    let segment = segment.trim_matches('/');
    if segment.is_empty() {
        return if base.is_empty() { "/".into() } else { base.to_string() };
    }
    if base.is_empty() {
        return format!("/{segment}");
    }
    format!("{base}/{segment}")
}

/// Resolve a method-level route template against class context.
///
/// Absolute templates (leading `/`) ignore the class base path.
pub fn resolve_handler_route(
    class_base_path: &str,
    method_template: &str,
    class_name: &str,
    method_name: &str,
) -> String {
    let resolved = resolve_method_route_path(method_template, class_name, method_name);
    if method_template.starts_with('/') {
        join_route_paths("", &resolved.trim_start_matches('/'))
    } else {
        join_route_paths(class_base_path, &resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_module_suffix_and_lowercases() {
        assert_eq!(api_resource_name("ProductsModule"), "products");
        assert_eq!(api_resource_name("MyFirstMODULE"), "myfirst");
        assert_eq!(api_resource_name("Health"), "health");
    }

    #[test]
    fn expands_module_token() {
        assert_eq!(
            resolve_route_path("/test/[module]", "ProductsModule"),
            "/test/products"
        );
        assert_eq!(
            resolve_route_path("/api/[module]/{id}", "UserModule"),
            "/api/user/{id}"
        );
    }

    #[test]
    fn expands_action_token() {
        assert_eq!(api_action_name("UserAction"), "user");
        assert_eq!(api_action_name("ListItems"), "listitems");
        assert_eq!(
            resolve_method_route_path("test/[action]", "UserModule", "UserAction"),
            "test/user"
        );
    }

    #[test]
    fn joins_class_and_method_routes() {
        assert_eq!(
            resolve_handler_route("/api/user", "test/[action]", "UserModule", "UserAction"),
            "/api/user/test/user"
        );
        assert_eq!(
            resolve_handler_route("/api/user", "/absolute/[action]", "UserModule", "UserAction"),
            "/absolute/user"
        );
    }
}
