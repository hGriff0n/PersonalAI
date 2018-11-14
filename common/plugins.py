#!/usr/bin/env python3

import abc
import asyncio
import clg
import imp
import inspect
import os
import queue
import uuid
import yaml

from common import logger


LoadedPlugin = None
_action_handles = {}


class Plugin:
    """
    Base class for all plugins. Singleton instances of subclasses are created automatically by loader.py
    """

    # List of system reserved actions
    # Users cannot register handles for these actions
    RESERVED_ACTIONS = []

    def __init__(self, logger, config=None):
        self._validate_configuration(config)
        self._config = config
        self._log = logger

        self._role = None
        self._uuid = str(uuid.uuid4())

        # NOTE: We use `self.__class__` to enable overloading the handler methods in subclasses
        # Using `Plugin.handle_ack` would restrict this to calling the default plugin handle
        self._register_handle('ack', self.__class__.handle_ack)
        self._register_handle('error', self.__class__.handle_error)

    # NOTE: This is implicitly called when we import in the subclass
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global LoadedPlugin
        LoadedPlugin = cls

    def _validate_configuration(self, config):
        """
        Read through the provided configuration dictionary and throw an exception if any values are in error

        Override to fit the plugin specific configuration
        """
        pass

    # Plugin interface
    @abc.abstractmethod
    async def run(self, comm):
        """
        Direct interfacing methdod for running the basic plugin behavior.

        NOTE: Plugins **must** implement this method as a single pass returning a boolean
        This method will be called repeatedly by the plugin loader, as-if it was continuously running

        :param comm: interface for sending messages, etc.
        :returns: A boolean value indicating whether to continue running the plugin or not
        """

    async def handle_unknown_message(self, msg, comm):
        """
        Callback for any received messages that do not have registered handles
        """
        self._log.info("Received unexpected message <{}>".format(msg.json_packet))

    async def handle_ack(self, msg, _comm):
        """
        Callback for any "acknowledge" messages sent to this plugin

        These messages generally provide feedback about the state of the request
        """
        self._log.debug("Handled ack message: {}".format(msg.id))

    async def handle_error(self, msg, _comm):
        """
        Callback for any "error" messages sent to this plugin

        These messages are sent if a request/message fails for some reason
        """
        self._log.error("Handled error message: {}".format(msg.resp))

    # Plugin registration
    def _register_handle(self, action, callback):
        """
        Registers the specific callback for all messages that have the indicated 'action'

        :param action: The action that this handle is registered for
        :param callback: A coroutine callback taking `(self, Message, CommChannel)`
        """
        global _action_handles
        if inspect.iscoroutinefunction(callback):
            if action not in Plugin.RESERVED_ACTIONS:
                _action_handles[str(action)] = callback
            else:
                self._log.error("Attempt to register user callback for reserved action `{}` ({})".format(action, callback))

        else:
            self._log.error("Attempt to register non-coroutine callback for `{}` action ({})".format(action, callback))

    # Properties
    @property
    def uuid(self):
        return self._uuid

    @property
    def role(self):
        return self._role

    @property
    def logger(self):
        return self._log


# TODO: Is there a way to move this into a yaml file??
def _get_plugin_default_arg_config():
    return {
        'options': {
            'log-level': {
                'help': "Specify the level for logging messages"
            }
        }
    }

def load(desired_plugin, log=None, args=None, plugin_dir=None, log_dir=None):
    if plugin_dir is None:
        log.error("plugin_dir was not specified")
        return

    # Make sure the plugin exists
    plugin_location = os.path.join(plugin_dir, desired_plugin)
    if not os.path.isdir(plugin_location) or not "__init__.py" in os.listdir(plugin_location):
        if log is not None:
            log.error("Could not find plugin {}".format(desired_plugin))
        return None

    # Load the plugin argument config file
    yaml_config_def = _get_plugin_default_arg_config()
    yaml_config_path = os.path.join(plugin_location, "config_def.yaml")
    if os.path.exists(yaml_config_path):
        with open(yaml_config_path) as yaml_file:
            try:
                plugin_config = yaml.safe_load(yaml_file)
                # Merge the user config into the default config
                for k, v in plugin_config.items():
                    yaml_config_def[k] = {
                        **yaml_config_def.get(k, {}), **v
                    }
            except Exception as e:
                log.error("Error loading plugin configuration for {}, assuming empty file: {}".format(desired_plugin, e))
    log.info(yaml_config_def)

    plugin_config_args = {}
    try:
        plugin_config_args = vars(clg.CommandLine(yaml_config_def).parse(args or []))
    except Exception as e:
        log.error("Error loading plugin configuration for {}, assuming no configuration: {}".format(desired_plugin, e))

    # Create the plugin specific logger
    log.debug("Setting plugin logger level={}".format(plugin_config_args.get('log-level')))
    plugin_logger = logger.create("{}.log".format(desired_plugin), log_dir=log_dir, level=plugin_config_args.pop('log-level', None))

    # Load the plugin
    log.info("Loading plugin module {}".format(desired_plugin))
    info = imp.find_module('__init__', [plugin_location])
    imp.load_module('__init__', *info)
    log.info("Loaded plugin {}".format(desired_plugin))

    # Construct the plugin
    global _action_handles
    plugin = LoadedPlugin(plugin_logger, plugin_config_args)
    handles = _action_handles
    _action_handles = {}

    return plugin, handles
