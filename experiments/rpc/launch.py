
# standard imports
import argparse
import itertools
import os
import re
import subprocess
import sys
import types
import typing
import yaml

# third-party imports

# local imports


def build(args, conf):
    """
    Build the device-manager server

    TODO: Should this also build the python sutff?
    """
    _unused = args

    launch_dir = os.getcwd()

    os.chdir(conf.get('device_manager', {}).pop('src'))
    ret = os.system("cargo build")
    os.chdir(launch_dir)
    return ret

def build_python_libs(args, conf):
    """
    Build and install the python client module

    NOTE: This isn't really applicable for deployment, only for development
    TODO: Produce a system which will check for the existence of the "{lang}" modules, and download+install if not
    """
    _unused = (args, conf) # Unused for now
    launch_dir = os.getcwd()

    # TODO: This should probably be specified in the config
    os.chdir('client')
    ret = os.system('cmd /c "python setup.py build && python setup.py install --user"')
    os.chdir(launch_dir)
    return ret

def clean(args, conf):
    """
    Clear out any "cruft" directories that may accumulate files while running

    This is particularly useful to clean out the log directory
    """
    _unused = args

    if 'log-dir' not in conf:
        conf['log-dir'] = "./log"

    for root, _dirs, files in os.walk(conf['log-dir']):
        for f in files:
            log_file = os.path.join(root, f)
            print("Removing log file `{}`".format(log_file))
            os.unlink(log_file)

def escape_arg(arg) -> str:
    """
    Convert arguments into "command-line acceptable" forms (quoting strings with spaces and comma-joining lists)
    """
    # TODO: Need a way to escape strings
    if isinstance(arg, list) or isinstance(arg, types.GeneratorType):
        return ','.join(arg)
    return str(arg)

def spawn_with_args(exe_path: str, *args, **kwargs):
    """
    Spawn the given executable with the passed args and kwargs

    args are passed as commands while kwargs are translated into cli flags
    """
    program = [ exe_path ]
    create_shell = kwargs.pop('cli_shell', False)

    program.extend(escape_arg(arg) for arg in args)
    program.extend("--{}={}".format(k, escape_arg(arg)) for k, arg in kwargs.items())

    print("Running {}".format(program))
    if create_shell:
        return subprocess.Popen(program, shell=True)
    return subprocess.Popen(program, shell=False, stdout=subprocess.PIPE)

modality_procs = []
def launch_modality(args, conf):
    """
    Launch the python modality specified by the config

    TODO: Abstract the configuration process to handle "non-python" clients?
    NOTE: Modality configuration must come through the config file (overriding from command line is infeasible in multi-modality launches)
    """
    _unused = args
    system_args = []
    if 'server_address' in conf:
        system_args.append("--server_address={}".format(conf.get('server_address')))
    if 'log_dir' in conf:
        system_args.append("--log_dir={}".format(conf.get('log_dir')))

    for modality in conf.pop('modalities', []):
        name = modality.pop('name')

        # Extract any loader-specific configuration for the modality (this cannot be overridden from the command line)
        loader = modality.pop('loader')
        loader_path = loader.pop('program')
        loader_args = [loader.pop('source')] + system_args
        loader_args.extend("--{}={}".format(k, escape_arg(v)) for k, v in loader.items())

        modality_procs.append(spawn_with_args(loader_path, *filter(None, loader_args), name, **modality))

manager_proc = None
def launch_server(args, conf):
    """
    Launch the server specified by the configs

    TODO: Only one of these should be runnable per device, figure out how to prevent that
    """
    _unused = args

    manager = conf.get('device_manager')
    if manager is None:
        print("Trying to launch device manager without launch configuration given")
        return 1

    print("Launching the device manager on address: {}".format(manager.get('addr')))
    manager_exe = manager.pop('path')
    manager.pop('src', None)
    manager.pop('stdio_log', None)
    manager.pop('cli_shell', False)

    global manager_proc
    manager_proc = spawn_with_args(manager_exe, manager)

def set_debug(debug):
    """
    Initialize the surrounding environment to provide debug information
    """
    def _debug(args, conf):
        _unused = args

        os.environ["RUST_BACKTRACE"] = (debug and "1" or "0")

    return _debug

# Allowed commands
COMMANDS = {
    'clean': clean,
    'debug': set_debug(True),
    'build': build,
    'install': build_python_libs,
    'launch': launch_modality,
    'serve': launch_server,
}

def main(args, conf):
    # run all of the specified commands
    # TODO: This won't work for passing arguments to the modalities/etc. through the command line
    for mode in conf.pop('commands', []):
        if mode in COMMANDS:
            ret = COMMANDS[mode](args, conf)
            if ret is not None and ret != 0:
                return ret
        else:
            print("Unrecognized launch command `{}`".format(mode))

    # If we spawned the device manager, then wait for it to end
    # Once we exit, kill any processes that we also spawned
    global manager_proc
    global modality_procs
    try:
        if manager_proc is not None:
            manager_proc.wait()

        # Otherwise, if we spawned any modalities, wait for them
        elif modality_procs:
            for proc in modality_procs:
                proc.wait()

    finally:
        if manager_proc is not None:
            manager_proc.kill()
        for proc in modality_procs:
            proc.kill()

if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument('-c', default="./conf.yaml", help="config file path")
    p.add_argument('--log_dir', help="Base directory for local log storage")
    p.add_argument('--server_address', help="IP address that the device manager should be listening on")

    # Parse all of the known arguments (defined above)
    conf, args = p.parse_known_args()
    cmd_conf = vars(conf)

    # Load any unset values from the config file (cli trumps config, except for 'commands')
    # We have to be careful to remove unset cli arguments as they will always end up overriding the configuration
    # TODO: This should probably extend to default values too
    config_file_path = cmd_conf.pop('c')
    with open(config_file_path) as f:
        conf = {**yaml.safe_load(f), **dict(filter(lambda e: e[1] is not None, cmd_conf.items()))}

    # Allow the specification of a bunch of a sequence of commands to run in a way
    # That still allows passing "positional" arguments to the modality/loader
    # argparse would seem to work by setting the 'choices' parameter, however that will
    # Throw an exception on the first item that "fails" the choices check.
    # Additionally, we only want to take commands until the first "non-command" item
    conf['commands'] = list(itertools.takewhile(lambda arg: arg in COMMANDS, args))
    del args[:len(conf['commands'])]

    main(args, conf)
