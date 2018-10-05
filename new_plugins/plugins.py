#!/usr/bin/env python3

import abc
import asyncio
import imp
import os
import queue
import uuid

import clg
import yaml

from common import logger


LoadedPlugin = None
action_handles = {}


class MessageEvent(asyncio.Event):
    value = None


class Plugin:
    """
    Base class for all plugins. Singleton instances of subclasses are created automatically by loader.py
    """

    def __init__(self, logger, config):
        self._uuid = uuid.uuid4()

    # NOTE: This is implicitly called when we import in the subclass, creating the instance
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global LoadedPlugin
        LoadedPlugin = cls

    # TODO: Need to work on the interfaces (how to send messages?)
    @abc.abstractmethod
    def run(self, comm):
        """
        Direct interfacing methdod for running the basic plugin behavior.

        NOTE: Plugins must implement this method as a single pass returning a boolean
        They should not handle "continuous" running, the loading framework handles this for them
        This is implemented so that the plugin does not hang if the server connection
        Is closed, but that fact is communicated (or used) properly within this method

        :param self:
        :param comm: interface for sending messages, etc.
        :returns: A boolean value indicating whether to continue running the plugin or not
        """

    @staticmethod
    def _register_handle(action, callback):
        global action_handles
        action_handles[str(action)] = callback


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
    log.setLevel(logger.logging.INFO)

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
    global action_handles
    plugin = LoadedPlugin(plugin_logger, plugin_config_args)
    handles = action_handles
    action_handles = {}

    return plugin, handles
