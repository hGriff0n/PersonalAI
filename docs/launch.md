
# Launching the System

In order to launch the network, an initial "system overseer" must be launched and running. Once this overseer is listening on it's specified tcp socket, nodes may connect and register themselves into the broader network. NOTE: At present, this overseer does not exist and is not required.

The process of bringing nodes online is very similar. The "device manager" must be launched and running first, listening on it's specified tcp port (it must listen listen on the `localhost` address). Once it is running, the device-manager will handle device registration with the broader network. After it is running, apps may then register themselves with the device manager (and the broader network), performing a standard handshake procedure in order to verify the app and to register app handles.

I also provide a 'launch.py' script to automate this procedure down to `python launch.py device`. Additional commands also exist to clean the logging directory and to build and install the python/rust packages (when performed in src).

## System configuration

## App configuration
