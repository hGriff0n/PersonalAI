
# Reimport stuff from submodules that should be in the 'plugins' scope
from personal_ai.plugins.client import Client
from personal_ai.plugins.loader import initialize_registered_plugins
from personal_ai.plugins.plugin import Plugin
from personal_ai.plugins.service import Service
