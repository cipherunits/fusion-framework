# fusion-ffi

Thin C ABI over [`fusion-core`](../fusion-core) for C# and other FFI consumers.

## Build

```bash
cargo build -p fusion-ffi
# → target/debug/libfusion_ffi.so | fusion_ffi.dll | libfusion_ffi.dylib
```

Optional: `export FUSION_FFI_PATH=/path/to/libfusion_ffi.so`

## Usage

Prefer the managed NuGet package **Fusion-Framework** (namespace `FusionFramework`): [`bindings/csharp/FusionFramework`](../../bindings/csharp/FusionFramework).

Low-level surface: `fusion_app_*`, `fusion_settings_*`, `fusion_string_dup` / `fusion_string_free`, route helpers. Handler callbacks must return strings allocated with `fusion_string_dup`.

## Docs

C# guides:  
**https://fusion.cipherunit.xyz/en/docs/csharp/v1**

- Site: [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- GitHub: [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)

## License

BSD 3-Clause
