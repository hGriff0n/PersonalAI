#!/usr/bin/env python3

import abc
import imp
import os

import clg
import yaml

from common import logger


loaded_plugin = None
plugin_config_args = None
plugin_loger = None


class Plugin:
    """
    Base class for all plugins. Singleton instances of subclasses are created automatically by loader.py
    """
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global loaded_plugin
        global plugin_logger
        loaded_plugin = cls(plugin_logger, plugin_config_args)

    @abc.abstractmethod
    def run(self, queue):
        """
        Direct interfacing methdod for running the basic plugin behavior.

        NOTE: Plugins must implement this method as a single pass returning a boolean
        They should not handle "continuous" running, the loading framework handles this for them
        This is implemented so that the plugin does not hang if the server connection
        Is closed, but that fact is communicated (or used) properly within this method

        :param self:
        :param queue: Communication queue for sending server messages
            NOTE: Use the `Message` package to place messages onto this queue (or "quit" for ending)
            NOTE: Do not read from the queue (messages are automatically popped by the writer thread)
        :returns: A boolean value indicating whether to continue running the plugin or not
        """
        return True

    def dispatch(self, msg, queue):
        """
        Callback that is invoked for every message that the plugin receives from the connected server

        NOTE: Plugins must implement this message as a single pass, otherwise the reading thread hangs

        :param self:
        :param msg: Json message that was received from the server
        :param queue: Communication queue for sending server messages
            NOTE: Use the `Message` package to place messages onto this queue (or "quit" for ending)
            NOTE: Do not read from the queue (messages are automatically popped by the writer thread)
        :returns: A boolean value indicating whether to continue reading from the server
        """
        return True

    def get_hooks(self):
        """
        Method used for determining what capabilities the plugin supports

        NOTE: This method will change a lot in the near future

        :param self:
        :returns: A list indicating the subscribed "modalities" that this plugin provides
        """
        return []


def load(desired_plugin, log=None, args=None, plugin_dir=None, log_dir=None):
    # Make sure the plugin exists
    if plugin_dir is None: plugin_dir = r"C:\Users\ghoop\Desktop\PersonalAI\modalities\plugins"
    location = os.path.join(plugin_dir, desired_plugin)
    if not os.path.isdir(location) or not "__init__.py" in os.listdir(location):
        if log is not None:
            log.info("Could not find plugin {}".format(desired_plugin))
        return None

    # Create the plugin's logger
    global plugin_logger
    plugin_logger = logger.create('{}.log'.format(desired_plugin), log_dir=log_dir)
    log.setLevel(logger.logging.INFO)

    # Load the command parser
    if args is None: args = []
    arg_yaml = os.path.join(location, 'cmd_args.yaml')
    if os.path.exists(arg_yaml):
        global plugin_config_args
        cmd = clg.CommandLine(yaml.load(open(arg_yaml)))
        plugin_config_args = vars(cmd.parse(args))

    # Load the plugin
    log.info("Loading plugin {}".format(desired_plugin))
    info = imp.find_module("__init__", [location])
    imp.load_module("__init__", *info)
    log.info("Loaded plugin {}".format(desired_plugin))

    return loaded_plugin

# API Documentation:
#   python-clg: https://github.com/fmenabe/python-clg
