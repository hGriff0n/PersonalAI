
# standard imports
import asyncio
# import queue
# import socket
import typing

# third-part imports

# local imports
import communication
import dispatcher
from rpc import typing as rpc_types
from rpc import Message, registration


# TODO: There might be a better way to provide the communication handler
class Plugin(rpc_types.PluginBase):
    """
    """

    def __init__(self, comm: communication.CommunicationHandler) -> None:
        self._comm = comm

    # This function is used by clients to perform actions and request information from the network
    # NOTE: AppServers are implemented through a separate dispatch system
    # NOTE: It's entirely possible for AppServers to define `main` and implement some processing there
    async def main(self) -> bool:
        await asyncio.sleep(5)
        return True


class AppServer(Plugin):
    """
    """

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
        endpoints = [endpoint for _, endpoint in registration.endpoints_for_class(type(self)).items()]
        resp = await self._comm.await_Response(Message(call="register_app", args={ 'handles': endpoints }, msg_id="foo"))

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

            # TODO: Explicit `deregister` is not implemented
            # deregister_id = "foo"
            # self._comm.send(rpc.Message(call="deregister_app", args={'handles': registered_handles}, msg_id="foo"))
            # self._comm.drop_message(deregister_id)
            return False

        # Add any "registered" endpoints to the dispatcher
        for endpoint in registered_handles:
            dispatcher.register_endpoint(endpoint, self)
        return True
