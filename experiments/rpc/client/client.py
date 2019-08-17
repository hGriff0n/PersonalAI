
# standard imports
import asyncio
import queue
import socket
import threading
import time
import typing

# third-part imports

# local imports
import communication
import dispatcher
import rpc
import plugins
import protocol


# TODO: Move this to a better position
DispatcherMap = typing.Dict[str, typing.Callable[[rpc.Message], typing.Coroutine[typing.Any, typing.Any, rpc.Message]]]

# Number of seconds to wait in the threads for the done signal to be set
SIGNAL_TIMEOUT = 0.3


# NOTE: This is provided by the library/loader
READER_TIMEOUT = communication.NetworkQueue.SOCKET_TIMEOUT
def reader(conn: communication.NetworkQueue,
           comm: communication.CommunicationHandler,
           loop: asyncio.AbstractEventLoop,
           done_signal: threading.Event,
           logger: typing.Optional[typing.Any] = None) -> None:
    """
    Thread callback to handle messages as they are received by the plugin
    """
    try:
        while True:
            msg = conn.get_message()

            # This error is already handled
            if msg is None:
                if done_signal.wait(SIGNAL_TIMEOUT):
                    return None
                continue

            if msg.msg_id in comm.waiting_messages:
                if logger is not None:
                    logger.info("Received response to message id={}".format(msg.msg_id))
                comm.waiting_messages[msg.msg_id].value = msg
                loop.call_soon_threadsafe(comm.waiting_messages[msg.msg_id].set)

            else:
                dispatch = dispatcher.get_dispatch_routine(msg.call)
                if not dispatch:
                    if logger is not None:
                        logger.warning("Received unexpected message {}: No endpoint registered for {}".format(msg.msg_id, msg.call))
                else:
                    if logger is not None:
                        logger.info("Handling message id={} through plugin handle".format(msg.msg_id))
                    asyncio.run_coroutine_threadsafe(dispatch(msg), loop=loop)

    except ConnectionResetError as e:
        if logger is not None:
            logger.error("Lost connection to server: {}".format(e))

    except Exception as e:
        if logger is not None:
            logger.error("Unexpected exception while waiting for messages: {}".format(e))

    print("Setting done signal: reader")
    done_signal.set()


# NOTE: This is provided by the library/loader
WRITER_TIMEOUT = communication.NetworkQueue.SOCKET_TIMEOUT
def writer(conn: communication.NetworkQueue,
           write_queue: 'queue.Queue[rpc.Message]',
           done_signal: threading.Event,
           logger: typing.Optional[typing.Any] = None) -> None:
    """
    Thread callback responsible for sending messages out of the plugin

    This enables us to avoid waiting on the write queue as it not an "async boundary"
    """
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
            if logger is not None:
                logger.error("Unexpected exception in writer thread: {}".format(e))

            print("Setting done signal: writer - {}".format(e))
            done_signal.set()
            return None


class Client(plugins.Client):

    async def main(self) -> bool:
        await asyncio.sleep(10)

        rpc_message = rpc.Message(call="grab_a_message")
        print("Calling `grab_a_message`")
        resp = await self._comm.wait_response(rpc_message)
        if resp is not None:
            print("Received {}".format(resp.resp))
        else:
            print("None")

        return False


class NullMessage(rpc.Serializable):

    def __init__(self):
        self.message: str = ""

    def serialize(self) -> rpc.SerializedMessage:
        return {
            'message': self.message
        }

    def deserialize(self, msg_dict: rpc.SerializedMessage) -> bool:
        self.message = msg_dict.get('message', '')
        return True


@rpc.service
class AppService(plugins.AppServer):

    @rpc.endpoint
    async def grab_a_message(self, msg: NullMessage) -> NullMessage:
        print("Returning a special message")
        msg.message = "This is a special message"
        return msg


# TODO: Work on the loading procedure
# TODO: Incorporate configuration into this
# TODO: Handle cases where loading fails (What do I mean by this?)
@typing.no_type_check
def load_all_services(comm: communication.CommunicationHandler, client: typing.Optional[typing.Type[Client]] = None) -> typing.List[plugins.Plugin]:
    services: typing.List[plugins.Plugin] = [  # The `issubclass` is required for typing
        plugin(comm) for plugin in rpc.registration.get_registered_services() if issubclass(plugin, plugins.Plugin)
    ]
    if client is not None:
        services.append(client(comm))
    return services


# NOTE: As soon as one plugin "completes", all plugins in the modality will "exit" (reader+writer will shut down)
# Modalities should be split up into chunks that follow this behavior
# ie. If two plugins should continue running if the other fails, that should be represented as 2 modalities
PLUGIN_SLEEP = WRITER_TIMEOUT - SIGNAL_TIMEOUT - SIGNAL_TIMEOUT
async def run_plugins(plugins: typing.List[plugins.Plugin], done_signal: threading.Event) -> None:
    async def _signaller():
        while True:
            await asyncio.sleep(PLUGIN_SLEEP)
            if done_signal.wait(SIGNAL_TIMEOUT):
                return

    callbacks = [_signaller]
    for plugin in plugins:
        async def _runner():
            while await plugin.main():
                await asyncio.sleep(1)

            print("Setting done signal: {}".format(plugin))
            done_signal.set()

        callbacks.append(_runner)

    print("Callbacks: {}".format(callbacks))
    _done, pending = await asyncio.wait([runner() for runner in callbacks], return_when=asyncio.FIRST_COMPLETED)
    done_signal.set()

    print("Canelling {}".format(pending))
    for p in pending:
        p.cancel()

    # For some reason, cancelling the pending tasks throws an exception in this function. Handle that
    try:
        await asyncio.gather(*pending)
    except asyncio.CancelledError:
        pass


#
# Loader/Runner Code
#
proto = protocol.JsonProtocol(None)

# Connect to server
addr = "127.0.0.1:6142".split(':')
sock = socket.socket()
sock.connect((addr[0], int(addr[1])))
sock_handler = communication.NetworkQueue(sock, proto, None)

# Construct the communication handles
write_queue: 'queue.Queue[rpc.Message]' = queue.Queue()
comm = communication.CommunicationHandler(write_queue)

# Construct the read/write threads
loop = asyncio.get_event_loop()
done_signal = threading.Event()
read_thread = threading.Thread(target=reader, args=(sock_handler, comm, loop, done_signal))
write_thread = threading.Thread(target=writer, args=(sock_handler, write_queue, done_signal))

# Load the plugins
# plugins = load_all_services(comm, client=Client)
plugins = load_all_services(comm)
print("Loaded services: {}".format(plugins))

# Run the launcher
read_thread.start()
write_thread.start()
loop.run_until_complete(run_plugins(plugins, done_signal))

# Wait for one of the threads to exit (and then close everything done)
done_signal.wait()

print("Closing the socket connection....")
sock.shutdown(socket.SHUT_RDWR)        # So we don't produce 'ConnectionReset' errors in the host server
sock.close()

print("Waiting for the reading thread to finish...")
read_thread.join(READER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)

print("Waiting for the writing thread to finish...")
write_thread.join(WRITER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)  # Because of the write queue timeout delay

print("All threads have exited successfully")
