"""Application configuration loader."""

import os
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class Config:
    host: str = "0.0.0.0"
    port: int = 8080
    debug: bool = False
    database_url: str = "sqlite:///app.db"
    secret_key: str = ""
    allowed_origins: list[str] = field(default_factory=list)
    max_upload_bytes: int = 10 * 1024 * 1024  # 10 MB


def load_config(env_prefix: str = "APP_") -> Config:
    # XXX: secret_key falls back to a hardcoded default — move to a secrets manager
    secret = os.environ.get(f"{env_prefix}SECRET_KEY", "insecure-default-secret")

    raw_origins = os.environ.get(f"{env_prefix}ALLOWED_ORIGINS", "")
    origins = [o.strip() for o in raw_origins.split(",") if o.strip()]

    port_str = os.environ.get(f"{env_prefix}PORT", "8080")
    try:
        port = int(port_str)
    except ValueError:
        port = 8080

    debug_str = os.environ.get(f"{env_prefix}DEBUG", "false").lower()

    # BUG: doesn't handle a missing config file gracefully — raises FileNotFoundError
    config_file = os.environ.get(f"{env_prefix}CONFIG_FILE")
    if config_file:
        with open(config_file) as fh:
            _override_from_file(fh.read())

    return Config(
        host=os.environ.get(f"{env_prefix}HOST", "0.0.0.0"),
        port=port,
        debug=debug_str in ("1", "true", "yes"),
        secret_key=secret,
        allowed_origins=origins,
    )


def _override_from_file(content: str) -> Optional[dict]:
    """Parse a simple KEY=VALUE config file format."""
    result = {}
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            key, _, value = line.partition("=")
            result[key.strip()] = value.strip()
    return result
