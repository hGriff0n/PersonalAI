
import abc
import imp
import os

loaded_plugin = None
class Plugin:
    """Base class for all plugins. Singleton instances of subclasses are created automatically and stored in Plugin.plugins class field."""
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global loaded_plugin
        loaded_plugin = cls()

    @abc.abstractmethod
    def run(self, queue):
        """Direct interfacing method for running the basic plugin"""
        pass

    def dispatch(self, msg, queue):
        """Secondary method for sending messages to the plugin"""
        return True

    def get_hooks(self):
        """Method used for determining what capabilities the plugin supports (for system communication)"""
        return []


def load(desired_plugin):
    location = os.path.join("./plugins", desired_plugin)
    if not os.path.isdir(location) or not "__init__.py" in os.listdir(location):
        return None
    info = imp.find_module("__init__", [location])
    imp.load_module("__init__", *info)
    return loaded_plugin
