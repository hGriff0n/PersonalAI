#! /usr/bin/env/python3

from subprocess import Popen, PIPE
import sys

import anyconfig


# TODO: Use appdirs to automatically store the config file in system directories
    # https://github.com/ActiveState/appdirs

# Wrapper for automatically transforming a dict into process arguments and then spawning the process
def spawn_with_args(program, arg_dict, shell=None):
    if not isinstance(program, list):
        program = [ program ]

    program.extend('--{}={}'.format(k, v) for k, v in arg_dict.items())
    if shell is not None and shell:
        return Popen(program, shell=True)

    return Popen(program, shell=False, stdout=PIPE)

# Wrapper for simplifying plugin spawning
# Also automatically special cases the shell to only spawn for the cli plugin
def spawn_plugin(plugin, arg_dict):
    return spawn_with_args(['python', 'modalities/loader.py', plugin], arg_dict, plugin == 'cli')


def launch_device(config):
    # Launch the device manager
    manager = config['manager']
    manager_exe = '{}/device-manager.exe'.format(manager['path'])
    del manager['path']
    procs = [ spawn_with_args(manager_exe, manager)]


    # Split out the plugins (cause we need to special case the cli plugin (if it exists))
    plugins = config['plugins']
    cli = [plugins['cli']] if 'cli' in plugins else []
    plugins = { k:v for k, v in plugins.items() if 'cli' != k }

    # Launch all plugins
    for name, plugin in plugins.items():
        procs.append(spawn_plugin(name, plugin))

    # Spawn the cli plugin last cause this will "take over" the command line
    if len(cli) == 1:
        spawn_plugin('cli', cli).wait()

    print("Waiting for modalities")


    # Wait for the modalities to finish running
    for proc in procs:
        proc.wait()


def launch_ai_node(config):
    return


if __name__ == "__main__":
    argc = len(sys.argv)
    if argc < 2:
        print("Requires at least one cli argument")

    else:
        if argc < 3:
            sys.argv.append('conf.yaml')

        mode = sys.argv[1]
        if mode == "device":
            launch_device(anyconfig.load(sys.argv[2]))

        elif mode == "brain":
            launch_ai_node(anyconfig.load(sys.argv[2]))

        else:
            print("Unrecognized mode argument `{}`".format(mode))
