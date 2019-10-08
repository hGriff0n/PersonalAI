
# standard imports
import asyncio
import importlib.abc
import importlib.util
import os.path
import queue
import socket
import threading
import time
import typing

# third-party imports

# local imports
from personal_ai import communication
from personal_ai import dispatcher
from personal_ai import logger
from personal_ai import rpc
from personal_ai import plugins
from personal_ai.plugins import plugin as plugin_types
from personal_ai import protocol


# Number of seconds to wait in the threads for the done signal to be set
SIGNAL_TIMEOUT = 0.3


READER_TIMEOUT = communication.NetworkQueue.SOCKET_TIMEOUT
def reader(conn: communication.NetworkQueue,
           comm: communication.CommunicationHandler,
           loop: asyncio.AbstractEventLoop,
           done_signal: threading.Event) -> None:
    """
    Thread callback to handle messages as they are received by the plugin
    """
    log = conn.logger
    try:
        while True:
            msg = conn.get_message()

            # This error is already handled
            if msg is None:
                if done_signal.wait(SIGNAL_TIMEOUT):
                    return None
                continue

            if msg.resp is not None:
                if msg.msg_id in comm.waiting_messages:
                    log.info("Received response to message id={}".format(msg.msg_id))
                    comm.waiting_messages[msg.msg_id].value = msg
                    loop.call_soon_threadsafe(comm.waiting_messages[msg.msg_id].set)

                else:
                    log.debug("Received unexpected response to message {}".format(msg.msg_id))

            else:
                dispatch = dispatcher.get_dispatch_routine(msg.call)
                if not dispatch:
                    log.debug("Received unexpected message {}: No endpoint registered for {}".format(msg.msg_id, msg.call))
                else:
                    log.info("Handling message id={} through plugin handle".format(msg.msg_id))
                    asyncio.run_coroutine_threadsafe(dispatch(msg, comm), loop=loop)

    except ConnectionResetError as e:
        log.exception("Lost connection to server: {}".format(e))

    except Exception as e:
        log.exception("Unexpected exception while waiting for messages: {}".format(e))

    log.info("Setting done signal: reader")
    done_signal.set()


WRITER_TIMEOUT = communication.NetworkQueue.SOCKET_TIMEOUT
def writer(conn: communication.NetworkQueue,
           write_queue: 'queue.Queue[rpc.Message]',
           done_signal: threading.Event,) -> None:
    """
    Thread callback responsible for sending messages out of the plugin

    This enables us to avoid waiting on the write queue as it not an "async boundary"
    """
    log = conn.logger
    while True:
        try:
            msg = write_queue.get(timeout=WRITER_TIMEOUT)
            conn.send_message(msg)

        # Queue.get throws an exception everyt `WRITER_TIMEOUT` seconds
        # At that point, we check whether we're the only thread (aside from main) currently running
        # And use that as a proxy for when we should return
        except queue.Empty:
            if done_signal.wait(SIGNAL_TIMEOUT):
                return None

        except Exception as e:
            log.exception("Unexpected exception in writer thread: {}".format(e))

            log.info("Setting done signal: writer - {}".format(e))
            done_signal.set()
            return None


PLUGIN_SLEEP = WRITER_TIMEOUT - SIGNAL_TIMEOUT - SIGNAL_TIMEOUT
async def run_plugins(all_plugins: typing.List[plugins.Plugin], done_signal: threading.Event, log: logger.Logger) -> None:
    """
    Manage the running of modality classes in the asyncio event loop

    This spawns up an async runner that repeatedly calls the class's `main` method until an exception is
    Thrown or a falsey value is returned. As soon as any one class exits, this function will forcefully
    Close all functions in the modality (not responsible for closing the connection to the device manager).

    NOTE: If there are plugins that shouldn't exit whenever the other closes, they should be put in different modalities
    """

    # Create a small "plugin" that periodically checks whether the done_signal has been set
    # This enables the plugin runner to exit when the reader/writer sets the signal, even if no plugins would fail then
    class Signaller(plugins.Plugin):
        def __init__(self):
            pass

        async def main(self) -> bool:
            await asyncio.sleep(PLUGIN_SLEEP)
            return not done_signal.wait(SIGNAL_TIMEOUT)

    log.debug("Appending exit signaller to plugin list")
    all_plugins.append(Signaller())

    # Create a plugin runner that periodically runs the 'Plugin.main' entrypoint
    # Sets the `done_signal` once `main` returns False
    async def _runner(plugin):
        try:
            while await plugin.main():
                await asyncio.sleep(1)
        finally:
            log.debug("Plugin exited. Setting done signal to close threads")
            done_signal.set()

    # Spawn all plugins on the event loop, cancel the active ones when one exits
    log.debug("Spawning plugins: {}".format(all_plugins))
    done, pending = await asyncio.wait([_runner(plugin) for plugin in all_plugins], return_when=asyncio.FIRST_COMPLETED)

    log.debug("Plugins finished: {}".format(done))
    log.debug("Cancelling plugins: {}".format(pending))
    for p in pending:
        p.cancel()

    # For some reason, cancelling the pending tasks throws an exception in this function. Handle that
    try:
        await asyncio.gather(*pending)
    except asyncio.CancelledError:
        pass
    log.debug("Plugins cancelled")


def import_plugins(config: typing.Dict[str, str], log: logger.Logger) -> str:
    """
    Helper method to locate and import a plugin from the specified directory
    The directory to load from is stored in the `path` field of the config dict
    In the directory, a singular __init__.py file is required that loads in all of the modality classes

    TODO: Find a way to "locate" the plugin based of of the name only?
    """
    modality_name = config.pop('name')

    log.debug("importing from {}".format(config.get('path')))
    module_path = os.path.join(config.pop('path'), '__init__.py')
    spec = importlib.util.spec_from_file_location(modality_name, module_path)
    log.debug("Found spec: {}".format(spec))
    module = importlib.util.module_from_spec(spec)

    assert isinstance(spec.loader, importlib.abc.Loader)
    spec.loader.exec_module(module)

    return modality_name


def connect_to_server(server_address: str,
                      max_retries: int,
                      timeout: float,
                      log: logger.Logger) -> typing.Optional[socket.socket]:
    """
    Helper method around the initial device_manager connection

    TODO: Handle retries and connection timeouts
    """
    del max_retries
    del timeout

    addr = server_address.split(':')
    log.debug("Connecting to socket on {}".format(addr))

    sock = socket.socket()
    sock.connect((addr[0], int(addr[1])))
    log.info("Listening to socket on {}".format(addr))

    return sock

def wait_and_exit(done_signal: threading.Event,
                  sock: socket.socket,
                  read_thread: threading.Thread,
                  write_thread: threading.Thread,
                  log: logger.Logger) -> None:
    """
    Wait for the done signal to be triggered and then close of all the threads and connections
    """
    done_signal.wait()

    log.debug("Closing the socket connection....")
    sock.shutdown(socket.SHUT_RDWR)        # So we don't produce 'ConnectionReset' errors in the host server
    sock.close()

    log.debug("Waiting for the reading thread to finish...")
    read_thread.join(READER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)

    log.debug("Waiting for the writing thread to finish...")
    write_thread.join(WRITER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)  # Because of the write queue timeout delay

def initialize_modality(args: typing.List[str],
                        comm: communication.CommunicationHandler,
                        log: logger.Logger,
                        log_dir: str) -> typing.Optional[typing.List[plugin_types.Plugin]]:
    """
    Helper method around `initialize_loaded_modality` to handle possible exceptions from client constructors
    """
    try:
        return plugins.initialize_loaded_modality(args, comm, log, log_dir)

    except Exception as e:
        log.error("Failed to initialize plugins: {}".format(e))
        return None

def main(args: typing.List[str], conf: typing.Dict[str, typing.Any]):
    # Create protocol and log "global" objects
    proto = protocol.JsonProtocol()
    log_dir = os.path.normpath(conf.pop('log_dir'))
    log = logger.create("loader.log", name='__loader__', log_dir=log_dir)

    server_addr = conf.pop('server_address')
    sock = connect_to_server(server_addr, 0, 0, log)
    if sock is None:
        log.error("Failed to connect to server on socket: {}".format(server_addr))
        return

    # Load the modality
    modality_name = import_plugins(conf, log)
    modality_logger = logger.create("{}.log".format(modality_name), name=modality_name, log_dir=log_dir)

    # Construct the communication handles
    write_queue: 'queue.Queue[rpc.Message]' = queue.Queue()
    comm = communication.CommunicationHandler(write_queue, modality_logger)
    sock_handler = communication.NetworkQueue(sock, proto, modality_logger)

    # Construct the modality plugins
    # TODO: Pass in unparsed args to this function?
    modality_plugins = initialize_modality(args, comm, log, log_dir)
    if modality_plugins is None:
        return

    modality_logger.debug("Initialized modality plugins: {}".format(modality_plugins))
    log.info("Starting modality {}....".format(modality_name))

    # Construct the read/write threads
    async_loop = asyncio.get_event_loop()
    done_signal = threading.Event()
    read_thread = threading.Thread(target=reader, args=(sock_handler, comm, async_loop, done_signal))
    write_thread = threading.Thread(target=writer, args=(sock_handler, write_queue, done_signal))

    # Run the plugins and threads
    read_thread.start()
    write_thread.start()
    async_loop.run_until_complete(run_plugins(modality_plugins, done_signal, modality_logger))

    # Wait for one of the threads to exit (and then close everything done)
    wait_and_exit(done_signal, sock, read_thread, write_thread, log)
    log.info("Successfully closed modality: {}".format(modality_name))
    modality_logger.info("Modality closed successfully")

# TODO: Need a way to indicate that the logger should also print to the terminal
if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser()
    p.add_argument('name', help="Name of the modality being loaded")
    p.add_argument('--server_address', help="IP address that the device manager is listening on")
    p.add_argument('--log_dir', help='Base directory for local log storage')
    p.add_argument('--source', help="Base directory for the modality definition", dest='path')

    conf, args = p.parse_known_args()
    main(args, vars(conf))
