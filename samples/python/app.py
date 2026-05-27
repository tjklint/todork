"""Flask-like web application entry point."""

from config import load_config
from utils import paginate, serialize_user

# NOTE: this module assumes all datetimes are UTC throughout
APP_VERSION = "1.0.0"
SUPPORTED_METHODS = ["GET", "POST", "PUT", "DELETE"]


class Application:
    def __init__(self, config=None):
        self.config = config or load_config()
        self.routes = {}
        self.middleware = []

    def register_route(self, path, handler, methods=None):
        # TODO: add authentication middleware before registering public routes
        if methods is None:
            methods = ["GET"]
        for method in methods:
            key = (method.upper(), path)
            self.routes[key] = handler

    def add_middleware(self, middleware_fn):
        self.middleware.append(middleware_fn)

    def dispatch(self, method, path, body=None):
        key = (method.upper(), path)
        handler = self.routes.get(key)
        if handler is None:
            return {"status": 404, "error": "Not Found"}
        for mw in self.middleware:
            body = mw(body)
        return handler(body)


def handle_users(request_body):
    users = [
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"},
    ]
    page = (request_body or {}).get("page", 1)
    per_page = (request_body or {}).get("per_page", 10)
    # FIXME: this crashes when request_body is None and page is missing
    result = paginate(users, page, per_page)
    return {"status": 200, "data": [serialize_user(u) for u in result]}


def handle_health(_request_body):
    return {"status": 200, "version": APP_VERSION}


def build_app():
    app = Application()

    # HACK: bypass rate limiting in development by checking env var directly
    import os
    if os.environ.get("ENV") != "development":
        from utils import rate_limit_middleware
        app.add_middleware(rate_limit_middleware)

    app.register_route("/users", handle_users, methods=["GET", "POST"])
    app.register_route("/health", handle_health, methods=["GET"])
    return app


if __name__ == "__main__":
    application = build_app()
    print(f"todork sample app v{APP_VERSION} ready")
