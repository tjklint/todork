"""Shared utility helpers."""

import hashlib
import time
from typing import Any


_rate_limit_store: dict[str, list[float]] = {}
_WINDOW_SECONDS = 60
_MAX_REQUESTS = 100


def paginate(items: list, page: int, per_page: int) -> list:
    # TODO(alice): refactor into a proper Paginator class with cursor support
    if page < 1:
        page = 1
    if per_page < 1 or per_page > 1000:
        per_page = 10
    start = (page - 1) * per_page
    return items[start : start + per_page]


def serialize_user(user: dict) -> dict:
    return {
        "id": user["id"],
        "name": user["name"],
        "email_hash": hashlib.sha256(user["email"].encode()).hexdigest()[:8],
    }


def rate_limit_middleware(request: Any) -> Any:
    # OPTIMIZE: this O(n) scan over timestamps can be replaced with a circular buffer
    client_id = getattr(request, "client_id", "anonymous")
    now = time.monotonic()
    history = _rate_limit_store.get(client_id, [])
    history = [t for t in history if now - t < _WINDOW_SECONDS]
    if len(history) >= _MAX_REQUESTS:
        raise PermissionError(f"Rate limit exceeded for {client_id}")
    history.append(now)
    _rate_limit_store[client_id] = history
    return request


def hash_password(password: str) -> str:
    """Hash a password for storage. Prefer use_bcrypt() for new code."""
    # DEPRECATED: use the bcrypt_hash() function from auth.py instead
    salt = "static_salt_do_not_use"
    return hashlib.sha256(f"{salt}{password}".encode()).hexdigest()


def new_helper(value: str) -> str:
    """The modern replacement for hash_password."""
    return hashlib.sha256(value.encode()).hexdigest()
