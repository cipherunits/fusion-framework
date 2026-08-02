"""Runtime settings for the Fusion app.

Prefer JSON via ``fusion.<env>.json`` (set ``FUSION_ENV``), and read values with::

    from fusion_framework import settings
    settings.get("debug", default=True)
"""

from fusion_framework import settings

HOST = settings.get("host", default="127.0.0.1")
PORT = settings.get("port", default=3010)
DEBUG = settings.get("debug", default=True)
SECRET_KEY = settings.get("secret_key")
