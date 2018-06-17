#!/usr/bin/env python3

import abc
import imp
import os

import clg
import yaml

# TODO: Look at having methods to create messages
    # Would be helpful as communication gets more complicated to setup record-keeping details

loaded_plugin = None
plugin_config_args = None

class Plugin:
    """Base class for all plugins. Singleton instances of subclasses are created automatically and stored in Plugin.plugins class field."""
    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global loaded_plugin
        loaded_plugin = cls(plugin_config_args)

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


def load(desired_plugin, log=None, args=None, _dir=None):
    # Make sure the plugin exists
    if _dir is None: _dir = r"C:\Users\ghoop\Desktop\PersonalAI\modalities\plugins"
    location = os.path.join(_dir, desired_plugin)
    if not os.path.isdir(location) or not "__init__.py" in os.listdir(location):
        if log is not None: log.info("Could not find plugin {}".format(desired_plugin))
        return None

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
