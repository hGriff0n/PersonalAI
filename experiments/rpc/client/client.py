
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
from personal_ai import protocol


# Number of seconds to wait in the threads for the done signal to be set
SIGNAL_TIMEOUT = 0.3


# TODO: Move to `launch.py` when that is created
# NOTE: This is provided by the library/loader
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


# TODO: Move to `launch.py` when that is created
# NOTE: This is provided by the library/loader
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


# TODO: Move to `launch.py` when that is created
# NOTE: As soon as one plugin "completes", all plugins in the modality will "exit" (reader+writer will shut down)
# Modalities should be split up into chunks that follow this behavior
# ie. If two plugins should continue running if the other fails, that should be represented as 2 modalities
PLUGIN_SLEEP = WRITER_TIMEOUT - SIGNAL_TIMEOUT - SIGNAL_TIMEOUT
async def run_plugins(all_plugins: typing.List[plugins.Plugin], done_signal: threading.Event, log: logger.Logger) -> None:

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


# TODO: Incorporate with config when I've moved totally to a "loader.py" setup
def import_plugins(config: typing.Dict[str, str], log: logger.Logger) -> str:
    modality_name = config.pop('name')

    log.debug("importing from {}".format(config.get('path')))
    module_path = os.path.join(config.pop('path'), '__init__.py')
    spec = importlib.util.spec_from_file_location(modality_name, module_path)
    log.debug("Found spec: {}".format(spec))
    module = importlib.util.module_from_spec(spec)

    assert isinstance(spec.loader, importlib.abc.Loader)
    spec.loader.exec_module(module)

    return modality_name


#
# Loader/Runner Code
#
proto = protocol.JsonProtocol(None)
log_dir = r"C:\Users\ghoop\Desktop\PersonalAI\experiments\rpc\logs"
log = logger.create("loader.log", name='__loader__', log_dir=log_dir)

# Connect to server
# TODO: Add retry handling on connection errors
addr = "127.0.0.1:6142".split(':')
log.debug("Connecting to socket on {}".format(addr))
sock = socket.socket()
sock.connect((addr[0], int(addr[1])))
log.info("Listening to socket on {}".format(addr))

# Load the modality
config = {
    'name': "tester",
    'path': r"C:\Users\ghoop\Desktop\PersonalAI\experiments\rpc\client\modalities\tester"
}
modality_name = import_plugins(config, log)
modality_logger = logger.create("{}.log".format(modality_name), name=modality_name, log_dir=log_dir)

# Construct the communication handles
write_queue: 'queue.Queue[rpc.Message]' = queue.Queue()
comm = communication.CommunicationHandler(write_queue, modality_logger)
sock_handler = communication.NetworkQueue(sock, proto, modality_logger)

# Initialize the modality (ie. construct the plugins)
all_plugins = plugins.initialize_loaded_modality(log, comm, log_dir)
modality_logger.debug("Initialized modality: {}".format(all_plugins))

print("Starting modality now....")
log.info("Starting modality {}".format(modality_name))

# Construct the read/write threads
loop = asyncio.get_event_loop()
done_signal = threading.Event()
read_thread = threading.Thread(target=reader, args=(sock_handler, comm, loop, done_signal))
write_thread = threading.Thread(target=writer, args=(sock_handler, write_queue, done_signal))

# Run the launcher
read_thread.start()
write_thread.start()
loop.run_until_complete(run_plugins(all_plugins, done_signal, modality_logger))

# Wait for one of the threads to exit (and then close everything done)
done_signal.wait()

log.debug("Closing the socket connection....")
sock.shutdown(socket.SHUT_RDWR)        # So we don't produce 'ConnectionReset' errors in the host server
sock.close()

log.debug("Waiting for the reading thread to finish...")
read_thread.join(READER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)

log.debug("Waiting for the writing thread to finish...")
write_thread.join(WRITER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)  # Because of the write queue timeout delay

print("Modality exited successfully")
log.info("Closed modality: {}".format(modality_name))
modality_logger.info("Modality closed successfully")
