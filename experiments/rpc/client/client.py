
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
            done_signal.set()
            return None


class Client(plugins.Client):

    async def main(self) -> bool:
        rpc_msg_dict = {
            "call": "register_app",
            "args": {
                "handles": [
                    "tell_story",
                    "list_books"
                ]
            },
            "msg_id": "foo",
        }
        rpc_message = rpc.Message.from_dict(rpc_msg_dict)

        if rpc_message is None:
            print("Failed to parse {} into an rpc.Message".format(rpc_msg_dict))
            return False

        # Send message and wait response
        print("Send {} to server.....".format(rpc_message.serialize()))
        resp = await self._comm.wait_response(rpc_message)
        if resp is not None:
            print("Received {}".format(resp.serialize()))

        return False


class NullMessage(rpc.BaseMessage):
    def serialize(self) -> rpc.SerializedMessage:
        return {}

    def deserialize(self, msg_dict: rpc.SerializedMessage) -> bool:
        return True


# @rpc.service
class AppService(plugins.AppServer):

    @rpc.endpoint
    async def test_fn_type(self, msg: NullMessage) -> NullMessage:
        return NullMessage()


# TODO: Work on the loading procedure
# TODO: Incorporate configuration into this
# TODO: Handle cases where loading fails (What do I mean by this?)
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
def run_plugins(plugins: typing.List[plugins.Plugin], loop, done_signal: threading.Event) -> None:
    for plugin in plugins:
        async def _runner():
            while await plugin.main():
                await asyncio.sleep(1)

            done_signal.set()

        asyncio.run_coroutine_threadsafe(_runner(), loop=loop)


    while not done_signal.wait(SIGNAL_TIMEOUT):
        time.sleep(PLUGIN_SLEEP)


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

# Construct the plugin threads
plugins = load_all_services(comm, client=Client)
plugin_thread = threading.Thread(target=run_plugins, args=(plugins, loop, done_signal))

# Run the launcher
read_thread.start()
write_thread.start()
plugin_thread.start()

# Wait for one of the threads to exit (and then close everything done)
done_signal.wait()

print("Closing the socket connection....")
sock.shutdown(socket.SHUT_RDWR)        # So we don't produce 'ConnectionReset' errors in the host server
sock.close()

print("Waiting for the reading thread to finish...")
read_thread.join(READER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)

print("Waiting for the reading thread to finish...")
write_thread.join(WRITER_TIMEOUT + 2 * SIGNAL_TIMEOUT + 1)  # Because of the write queue timeout delay

print("Waiting for the plugins thread to finish...")
plugin_thread.join(PLUGIN_SLEEP + 2 * SIGNAL_TIMEOUT + 1)
print("All threads have exited successfully")
