# Integration tests (future)

HTTP-level tests that start a real `FusionApp` and hit endpoints with `httpx` or `curl` belong here.

Mark with `@pytest.mark.integration` and run:

```bash
pytest tests/python/integration -m integration
```
