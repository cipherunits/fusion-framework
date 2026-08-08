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

/// Expand the reserved `[module]` token using the module class name.
///
/// Example: `/api/[module]/{id}` + `ProductsModule` → `/api/products/{id}`.
/// Other `{param}` / `[param]` segments are left for the router.
pub fn resolve_route_path(template: &str, class_name: &str) -> String {
    template.replace("[module]", &api_resource_name(class_name))
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
}
