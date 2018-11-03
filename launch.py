#! /usr/bin/env/python3

import os
from subprocess import Popen, PIPE
import sys

import anyconfig


# TODO: Use appdirs to automatically store the config file in system directories
    # https://github.com/ActiveState/appdirs

# Wrapper for automatically transforming a dict into process arguments and then spawning the process
def spawn_with_args(program, prog_args, shell=None):
    if not isinstance(program, list):
        program = [ program ]

    if isinstance(prog_args, dict):
        program.extend('--{}={}'.format(k, v) for k, v in prog_args.items())
    elif isinstance(prog_args, list):
        program.extend(arg for arg in prog_args)
    elif isinstance(prog_args, str):
        program.append(prog_args)

    if shell:
        return Popen(program, shell=True)

    return Popen(program, shell=False, stdout=PIPE)

# Wrapper for simplifying plugin spawning
# Also automatically special cases the shell to only spawn for the cli plugin
def spawn_plugin(plugin, arg_dict, loader):
    loader_path = loader.pop('script-path')

    # Merge the command arguments
    # NOTE: This overwrites any plugin specific args with the loader args where clashes occur
    args = list('--{}={}'.format(k, v) for k, v in loader.items())
    args.append(plugin)
    if arg_dict is not None:
        args.extend('--{}={}'.format(k, v) for k, v in arg_dict.items())

    print("Spawning the `{}` plugin".format(plugin))
    return spawn_with_args(['python', loader_path], args, plugin == 'cli')


def launch_device(config):
    procs = []

    # Launch the ai manager
    if 'ai-manager' in config:
        print("Launching the ai-manager")
        manager = config['ai-manager']
        manager_exe = manager.pop('path')
        manager.pop('src', None)
        manager.pop('stdio-log', None)
        procs.append(spawn_with_args(manager_exe, manager))

        # TODO: Spawn ai modalities


    # Launch the device manager
    print("Launching the device manager")
    manager = config['device_manager']

    if not _will_plugins_connect(manager['addr'], config['loader_config']):
        print("Configuration Error: Plugins not set to connect to local device manager")
        return 1

    manager_exe = manager.pop('path')
    manager.pop('src', None)
    manager.pop('stdio-log', None)
    procs.append(spawn_with_args(manager_exe, manager))

    # Split out the modalities (cause we need to special case the cli plugin (if it exists))
    # TODO: Shouldn't this be based on the folder structure (if it's a plugin system? - that may be bad in the future, i feel)
    plugins = config['plugins']
    cli = []
    if 'cli' in plugins:
        cli.append(plugins.pop('cli'))

    # Launch all plugins
    procs.extend(spawn_plugin(name, plugin, config['loader_config'].copy()) for name, plugin in plugins.items())

    # Spawn the cli plugin last cause this will "take over" the command line
    if len(cli) == 1:
        spawn_plugin('cli', cli[0], loader=config['loader_config']).wait()

    # Wait for the modalities to finish running
    print("Waiting for modalities")
    try:
        for proc in procs:
            proc.wait()

    except KeyboardInterrupt:
        for proc in procs:
            proc.kill()

def _will_plugins_connect(manager_addr, loader_config):
    return manager_addr == "127.0.0.1:{}".format(loader_config['port'])


def launch_ai_node(config):
    pass

def clean(config):
    if 'log-dir' not in config:
        print("Setting the `log-dir` config option to `./log`")
        config['log-dir'] = './log'

    for root, _dirs, files in os.walk(config['log-dir']):
        for f in files:
            log_file = os.path.join(root, f)
            print("Removing log file `{}`".format(log_file))
            os.unlink(log_file)

def build(config):
    launch_dir = os.getcwd()

    # os.chdir('ai-manager')
    # ret = os.system("cargo build")
    # os.chdir('..')

    # if ret == 0:
    if True:
        src_dir = config.get('device_manager', {}).get('src')
        if src_dir is not None:
            os.chdir(src_dir)
            ret = os.system("cargo build")
            os.chdir(launch_dir)
        else:
            print("Not building device manager: No src directory was specified")

    return ret

def build_python(_config):
    os.system("python setup.py build")
    os.system("python setup.py install --user")


def main(args, conf):
    commands = {
        'device': lambda: launch_device(conf),
        'brain': lambda: launch_ai_node(conf),
        'clean': lambda: clean(conf),
        'debug': lambda: os.system("$Env:RUST_BACKTRACE=1"),
        'build': lambda: build(conf),
        'install': lambda: build_python(conf)
    }

    for mode in sys.argv[1:]:
        if mode in commands:
            ret = commands[mode]()
            if ret is not None and ret != 0:
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
