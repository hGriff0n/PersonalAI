
# standard imports
import argparse
import os
import subprocess
import sys
import types
import typing
import yaml

# third-party imports

# local imports


def build(conf):
    """
    Build the device-manager server

    TODO: Should this also build the python sutff?
    """
    launch_dir = os.getcwd()

    os.chdir(conf.get('device_manager', {}).pop('src'))
    ret = os.system("cargo build")
    os.chdir(launch_dir)
    return ret

def build_python_libs(conf):
    """
    Build and install the python client module

    NOTE: This isn't really applicable for deployment, only for development
    """
    del conf  # Unused for now
    launch_dir = os.getcwd()

    # TODO: This should probably be specified in the config
    os.chdir('client')
    ret = os.system('cmd /c "python setup.py build && python setup.py install --user"')
    os.chdir(launch_dir)
    return ret

def clean(conf):
    """
    Clear out any "cruft" directories that may accumulate files while running

    This is particularly useful to clean out the log directory
    """
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
    arg = str(arg)
    # if ' ' in arg:
    #     return "\"{}\"".format(arg)
    return arg

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
def launch_modality(conf):
    """
    Launch the python modality specified by the config

    TODO: Work on this configuration a lot (This is kinda specific to python)
    """
    for modality in conf.pop('modalities', []):
        loader = modality.pop('loader')
        loader_path = loader.pop('program')
        loader_args = [loader.pop('source')]
        loader_args.extend("--{}={}".format(k, escape_arg(v)) for k, v in loader.items())

        name = modality.pop('name')
        modality_procs.append(spawn_with_args(loader_path, *filter(None, loader_args), name, **modality))

manager_proc = None
def launch_server(conf):
    """
    Launch the server specified by the configs

    TODO: Only one of these should be runnable per device, figure out how to prevent that
    """
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
    os.environ["RUST_BACKTRACE"] = (debug and "1" or "0")

def main(args, conf):
    commands = {
        'clean': lambda: clean(conf),
        'debug': lambda: set_debug(True),
        'build': lambda: build(conf),
        'install': lambda: build_python_libs(conf),
        'launch': lambda: launch_modality(conf),
        'serve': lambda: launch_server(conf),
    }

    # run all of the specified commands
    for mode in sys.argv[1:]:
        if mode in commands:
            ret = commands[mode]()
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
    p.add_argument('log_dir', help="Base directory for local log storage")
    p.add_argument('server_address', help="IP address that the server is listening on")

    # Parse all of the known arguments (defined above)
    conf, args = p.parse_known_args()
    conf = vars(conf)

    # Load any unset values from the config file (cli trumps config)
    config_file_path = conf.pop('c')
    with open(config_file_path) as f:
        conf = {**yaml.safe_load(f), **conf}

    main(args, conf)
