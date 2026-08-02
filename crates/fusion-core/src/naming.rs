//! Shared route naming helpers used by all language bindings.

/// Canonical HTTP handler method names (lowercase).
pub const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

/// `MyFirstApi` → `MyFirst`; names without an `Api`/`API` suffix are unchanged.
pub fn api_resource_name(class_name: &str) -> String {
    if let Some(stem) = class_name.strip_suffix("Api") {
        if !stem.is_empty() {
            return stem.to_string();
        }
    }
    if let Some(stem) = class_name.strip_suffix("API") {
        if !stem.is_empty() {
            return stem.to_string();
        }
    }
    class_name.to_string()
}

/// Expand the reserved `[name]` token using the API class name.
///
/// Example: `/api/[name]/{id}` + `ProductsApi` → `/api/Products/{id}`.
/// Other `{param}` / `[param]` segments are left for the router.
pub fn resolve_route_path(template: &str, class_name: &str) -> String {
    template.replace("[name]", &api_resource_name(class_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_api_suffix() {
        assert_eq!(api_resource_name("ProductsApi"), "Products");
        assert_eq!(api_resource_name("MyFirstAPI"), "MyFirst");
        assert_eq!(api_resource_name("Health"), "Health");
    }

    #[test]
    fn expands_name_token() {
        assert_eq!(
            resolve_route_path("/test/[name]", "ProductsApi"),
            "/test/Products"
        );
        assert_eq!(
            resolve_route_path("/api/[name]/{id}", "UserApi"),
            "/api/User/{id}"
        );
    }
}
