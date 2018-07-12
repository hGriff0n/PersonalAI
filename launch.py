#! /usr/bin/env/python3

import os
from subprocess import Popen, PIPE
import sys

import anyconfig


# TODO: Use appdirs to automatically store the config file in system directories
    # https://github.com/ActiveState/appdirs

# Wrapper for automatically transforming a dict into process arguments and then spawning the process
def spawn_with_args(program, arg_dict, shell=None):
    if not isinstance(program, list):
        program = [ program ]

    if arg_dict is not None:
        program.extend('--{}={}'.format(k, v) for k, v in arg_dict.items())

    if shell is not None and shell:
        return Popen(program, shell=True)

    return Popen(program, shell=False, stdout=PIPE)

# Wrapper for simplifying plugin spawning
# Also automatically special cases the shell to only spawn for the cli plugin
def spawn_plugin(plugin, arg_dict, loader=None):
    if loader is None:
        loader = 'modalities/loader.py'
    return spawn_with_args(['python', loader, plugin], arg_dict, plugin == 'cli')


def launch_device(config):
    procs = []

    # Launch the ai manager
    if 'ai-manager' in config:
        manager = config['ai-manager']
        manager_exe = manager['path']
        del manager['path']
        if 'stdio-log' in manager: del manager['stdio-log']
        procs.append(spawn_with_args(manager_exe, manager))

        # TODO: Spawn ai modalities


    # Launch the device manager
    manager = config['device-manager']
    manager_exe = manager['path']
    del manager['path']
    if 'stdio-log' in manager: del manager['stdio-log']
    procs.append(spawn_with_args(manager_exe, manager))


    # Split out the modalities (cause we need to special case the cli plugin (if it exists))
    # TODO: Shouldn't this be based on the folder structure (if it's a plugin system? - that may be bad in the future, i feel)
    plugins = config['plugins']
    cli = [plugins['cli']] if 'cli' in plugins else []
    del plugins['cli']

    # Launch all plugins
    procs.extend(spawn_plugin(name, plugin, loader=config['loader']) for name, plugin in plugins.items())

    # Spawn the cli plugin last cause this will "take over" the command line
    if len(cli) == 1:
        spawn_plugin('cli', cli[0], loader=config['loader']).wait()

    # Wait for the modalities to finish running
    print("Waiting for modalities")
    try:
        for proc in procs:
            proc.wait()

    except KeyboardInterrupt:
        for proc in procs:
            proc.kill()


def launch_ai_node(config):
    pass

def clean(config):
    if 'log-dir' not in config:
        config['log-dir'] = './log'

    for root, _dirs, files in os.walk(config['log-dir']):
        for f in files:
            os.unlink(os.path.join(root, f))

def build(_config):
    os.chdir('ai-manager')
    ret = os.system("cargo build")
    os.chdir('..')

    if ret == 0:
        os.chdir('device-manager')
        ret = os.system("cargo build")
        os.chdir('..')

    return ret


def main(args, conf):
    for mode in sys.argv[1:]:
        if mode == "device":
            launch_device(conf)

        elif mode == "brain":
            launch_ai_node(conf)

        elif mode == "clean":
            clean(conf)

        elif mode == "build":
            if build(conf) != 0:
                return

        else:
            print("Unrecognized mode argument `{}`".format(mode))

if __name__ == "__main__":
    argc = len(sys.argv)
    if argc < 2:
        print("Requires at least one cli argument")

    else:
        conf = anyconfig.load('conf.yaml')
        main(sys.argv[1:], conf)


# API Documentation:
#   anyconfig: https://github.com/ssato/python-anyconfig
