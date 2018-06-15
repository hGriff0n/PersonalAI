#! /usr/bin/env/python3

from subprocess import Popen, CREATE_NEW_CONSOLE
import sys

def launch_device():
    procs = [Popen(['device-manager/target/debug/device-manager.exe'])]

    # TODO: Integrate the config direction here
    procs.append(Popen(['python', './modalities/loader.py', 'dispatch']))
    procs.append(Popen(['python', './modalities/loader.py', 'audio']))

    # TODO: This needs to at least spawn a separate command window
    # NOTE: This actually spawns in the "launcher" window (for some reason)
    procs.append(Popen(['python', './modalities/loader.py', 'cli'], shell=True))

    for proc in procs:
        proc.wait()

    return

def launch_ai_node():
    return

def main(mode):
    if mode == "device":
        launch_device()

    elif mode == "brain":
        launch_ai_node()

    else:
        print("Unrecognized mode `{}`".format(mode))

if __name__ == "__main__":
    if len(sys.argv) == 2:
        main(sys.argv[1])

    else:
        print("Require one cli argument")
