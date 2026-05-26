# Earmark CLI Config Schema

The CLI supports a bounded TOML config surface.

Default location:
- `<root>/.earmark/config.toml` when `--root` is provided
- `./.earmark/config.toml` otherwise

Optional explicit location:
- `--config <path>`
- `EM_CONFIG=<path>`

Supported fields:

```toml
root = "/path/to/workspace"
default_system_id = "sys_research_synthesis"
json = true
log_level = "info"
provider_plugin_dirs = ["/path/to/shared/provider-plugins"]
```

Precedence:
1. Defaults
2. Config file
3. Environment variables
4. CLI arguments

Environment overrides:
- `EM_ROOT`
- `EM_SYSTEM_ID`
- `EM_JSON`
- `EM_LOG_LEVEL`
- `EM_PROVIDER_PLUGIN_DIRS` (colon-separated)

Provider plugin discovery always includes the workspace-local default directory:

- `<root>/.earmark/plugins/providers`

Additional plugin directories can be supplied through `provider_plugin_dirs` or `EM_PROVIDER_PLUGIN_DIRS`.
