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
        self._uuid = str(uuid.uuid4())
        self._log = logger
        _config = config

        self._register_handle('ack', self.handle_ack)
        self._register_handle('error', self.handle_error)

    # NOTE: This is implicitly called when we import in the subclass
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global LoadedPlugin
        LoadedPlugin = cls

    # TODO: Need to work on the interfaces (how to send messages?)
    @abc.abstractmethod
    async def run(self, comm):
        """
        Direct interfacing methdod for running the basic plugin behavior.

        NOTE: Plugins **must** implement this method as a single pass returning a boolean
        This method will be called repeatedly by the plugin loader, as-if it was continuously running

        :param comm: interface for sending messages, etc.
        :returns: A boolean value indicating whether to continue running the plugin or not
        """

    async def handle_ack(self, msg, _comm):
        """
        Callback for any "acknowledge" messages sent to this plugin

        These messages generally provide feedback about the state of the request
        """
        self._log.debug("ACK: {}".format(msg.id))

    async def handle_error(self, msg, _comm):
        """
        Callback for any "error" messages sent to this plugin

        These messages are sent if a request/message fails for some reason
        """
        self._log.error("ERROR: {}".format(msg.args))

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

    @property
    def uuid(self):
        return self._uuid


def load(desired_plugin, log=None, args=None, plugin_dir=None, log_dir=None):
    if plugin_dir is None:
        plugin_dir = r"C:\Users\ghoop\Desktop\PersonalAI\modalities\plugins"

    # Make sure the plugin exists
    location = os.path.join(plugin_dir, desired_plugin)
    if not os.path.isdir(location) or not "__init__.py" in os.listdir(location):
        if log is not None:
            log.info("Could not find plugin {}".format(desired_plugin))
        return None

    # Create the plugin specific logger
    plugin_logger = logger.create('{}.log'.format(desired_plugin), log_dir=log_dir)
    plugin_logger.setLevel(logger.logging.INFO)

    # Load the plugin arguments
    plugin_config_args = {}
    arg_yaml = os.path.join(location, 'cmd_args.yaml')
    if os.path.exists(arg_yaml):
        cmd = clg.CommandLine(yaml.load(open(arg_yaml)))
        plugin_config_args = vars(cmd.parse(args or []))

    # Load the plugin
    log.info("Loading plugin {}".format(desired_plugin))
    info = imp.find_module("__init__", [location])
    imp.load_module("__init__", *info)
    log.info("Loaded plugin {}".format(desired_plugin))

    # Construct the plugin
    global _action_handles
    plugin = LoadedPlugin(plugin_logger, plugin_config_args)
    handles = _action_handles
    _action_handles = {}

    return plugin, handles
