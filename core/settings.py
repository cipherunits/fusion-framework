# Fusion Framework Settings
# --------------------------------------------
# This file contains the core configuration
# for your application.
#
# License: MIT License
# You are free to use, modify, and distribute.


# variables or external config providers.
from fusion_framework import settings


# Never expose your secret key in public repositories
SECRET_KEY = settings.get("secret_key")

# Enable debug mode (DO NOT use True in production)
DEBUG = settings.get("debug", default=False)

HOST = settings.get("host", default="127.0.0.1")
PORT = settings.get("port", default=3000)
ENV = settings.env
CONFIG = settings.config
PROJECT_NAME = settings.get("project_name", default="fusion-app")
