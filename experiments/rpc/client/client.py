
# standard imports
import asyncio
import queue
import socket
import threading
import typing

# third-part imports

# local imports
import communication
import dispatcher
import rpc
import protocol


# TODO: Move this to a better position
DispatcherMap = typing.Dict[str, typing.Callable[[rpc.Message], typing.Coroutine[typing.Any, typing.Any, rpc.Message]]]


# NOTE: This is provided by the library/loader
def reader(conn: communication.NetworkQueue,
           comm: communication.CommunicationHandler,
           loop: asyncio.AbstractEventLoop,
           logger: typing.Optional[typing.Any] = None) -> None:
    """
    Thread callback to handle messages as they are received by the plugin
    """
    try:
        while True:
            msg = conn.get_message()

            # This error is already handled
            if msg is None:
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
            logger.error("Exception while waiting for messages: {}".format(e))


# NOTE: This is provided by the library/loader
WRITER_TIMEOUT = 5
def writer(conn: communication.NetworkQueue, write_queue: 'queue.Queue[rpc.Message]') -> None:
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
            num_active_threads = 0
            for t in threading.enumerate():
                if not (t is threading.main_thread()):
                    num_active_threads += 1
            if num_active_threads == 1:
                break


# TODO: Move to a "plugin" module?
# TODO: Might be a better way to provide the communication handler
class Plugin(rpc.PluginBase):

    def __init__(self, comm: communication.CommunicationHandler) -> None:
        self._comm = comm

    # This function is used by clients to perform actions and request information from the network
    # NOTE: AppServers are implemented through a separate dispatch system
    # NOTE: It's entirely possible for AppServers to define `main` and implement some processing there
    async def main(self) -> bool:
        await asyncio.sleep(5)
        return True

class Client(Plugin):

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

class AppServer(Plugin):

    REQUIRED_HANDLES: typing.ClassVar[typing.List[str]] = []

    def __init__(self, comm: communication.CommunicationHandler):
        super().__init__(comm)
        self._registered = False

    async def main(self) -> bool:
        if not self._registered:
            self._registered = await self._register()
            return self._registered

        return await self.run()

    async def run(self):
        await asyncio.sleep(5)
        return True

    # TODO: Generate the message ids
    async def _register(self):
        endpoints = [endpoint for _, endpoint in rpc.registration.endpoints_for_class(type(self)).items()]
        resp = await self._comm.await_Response(rpc.Message(call="register_app", args={ 'handles': endpoints }, msg_id="foo"))

        registered_handles = []
        if resp.resp and 'registered' in resp.resp:
            registered_handles.extend(resp.resp['registered'])

        # Check that all required handles are registered (if any)
        # If some handle is required but fails to register, then deregister the whole app
        unregistered_endpoints = [
            endpoint for endpoint in self.REQUIRED_HANDLES if endpoint not in registered_handles
        ]
        if len(unregistered_endpoints) > 0:
            print("Failure to register handles for service {}: {}".format(type(self), unregistered_endpoints))

            deregister_id = "foo"
            self._comm.send(rpc.Message(call="deregister_app", args={'handles': registered_handles}, msg_id="foo"))
            self._comm.drop_message(deregister_id)
            return False

        # Add any "registered" endpoints to the dispatcher
        for endpoint in registered_handles:
            dispatcher.register_endpoint(endpoint, self)
        return True


class NullMessage(rpc.BaseMessage):
    def serialize(self) -> rpc.SerializedMessage:
        return {}

    def deserialize(self, msg_dict: rpc.SerializedMessage) -> bool:
        return True


class AppService(Plugin):

    @rpc.endpoint
    async def test_fn_type(self, msg: NullMessage) -> NullMessage:
        return NullMessage()


# NOTE: This is provided by the library/loader
# Runtime function that manages the threads and main communication loop
# This is responsible for stopping when any thread exits
async def run_plugin(plugins: typing.List[Plugin],
                     sock,
                     read_thread: threading.Thread,
                     write_thread: threading.Thread):
    read_thread.start()
    write_thread.start()

    # TODO: Fix to allow for all plugins to be run
    plugin = plugins[-1]

    try:
        while True:
            finish_run = await plugin.main()

            if not finish_run:
                print("Stopping because plugin finished running")
                break

            if not write_thread.is_alive():
                print("Stopping because writer thread has stopped")
                break

            if not read_thread.is_alive():
                print("Stopping because reader thread has stopped")
                break

            await asyncio.sleep(5)
    except:
        print("EXCEPTION")

    # Close everything down
    sock.shutdown(socket.SHUT_RDWR)        # So we don't produce 'ConnectionReset' errors in the host server
    sock.close()
    read_thread.join()
    write_thread.join(WRITER_TIMEOUT + 1)  # Because of the write queue timeout delay


# TODO: Work on the loading procedure
# TODO: Incorporate configuration into this
# TODO: Handle cases where loading fails (What do I mean by this?)
def load_all_services(comm: communication.CommunicationHandler, client: typing.Optional[typing.Type[Client]] = None) -> typing.List[Plugin]:
    services: typing.List[Plugin] = [  # The `issubclass` is required for typing
        plugin(comm) for plugin in rpc.registration.get_registered_services() if issubclass(plugin, Plugin)
    ]
    if client is not None:
        services.append(client(comm))
    return services


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
read_thread = threading.Thread(target=reader, args=(sock_handler, comm, loop))
write_thread = threading.Thread(target=writer, args=(sock_handler, write_queue))

# Run the plugin
plugins = load_all_services(comm, client=Client)
loop.run_until_complete(run_plugin(plugins, sock, read_thread, write_thread))
print("All threads closed")
