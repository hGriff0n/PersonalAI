
# standard imports
import asyncio
import typing
# import uuid

# third-part imports

# local imports
import communication
import dispatcher
from rpc import typing as rpc_types
from rpc import Message, registration


# TODO: There might be a better way to provide the communication handler
class Plugin(rpc_types.PluginBase):
    """
    Base class for all plugins

    Exports the `main` method which acts as a customization point for any code that needs to be run every so often
    Clients will overload this method to implement their "calling" code
    """

    def __init__(self, comm: communication.CommunicationHandler) -> None:
        self._comm = comm

    async def main(self) -> bool:
        """
        Entrypoint for any code that needs to be run semi-regularly
        """
        await asyncio.sleep(5)
        return True


class Client(Plugin):
    """
    Specialization for clients
    TODO: Hopefully allow for automatic determination of the "Client" class (if any)
    """
    pass


class AppServer(Plugin):
    """
    Augment plugins with the capability to act as an app server
    App servers export rpc endpoints to the wider network, so that other clients can be called

    This class automatically defines `main` to do a "register_app" call, which exports it's endpoints
    To provide "main" service, overload the `run` method
    NOTE: Registration fails if any required endpoint is not registered
    """

    def __init__(self, comm: communication.CommunicationHandler):
        super().__init__(comm)
        self._registered = False

    async def main(self) -> bool:
        """
        Registers the server in the network and then
        """
        if not self._registered:
            self._registered = await self._register()
            return self._registered

        return await self.run()

    async def run(self):
        """
        Entrypoint for any code that app servers need to be run semi-regularly
        """
        await asyncio.sleep(5)
        return True

    # TODO: Generate the message ids
    async def _register(self):
        """
        Construct and perform the registration calls to the network

        Fails if any required handle is not registered
        """
        endpoints = registration.endpoints_for_class(type(self))
        handles = [handle for handle, _ in endpoints.items()]
        required = [handle for handle, endpoint in endpoints.items() if endpoint.get('required')]

        print("Registring handles for {}: {}".format(type(self), handles))
        resp = await self._comm.wait_response(Message(call="register_app", args={ 'handles': handles }))

        registered_handles = []
        if resp.resp and 'registered' in resp.resp:
            registered_handles.extend(resp.resp['registered'])
        print("Registered handles for service {}: {}".format(type(self), registered_handles))

        # Check that all required handles are registered (if any)
        # If some handle is required but fails to register, then deregister the whole app
        broken_endpoints = [handle for handle in required if handle not in registered_handles]
        if len(broken_endpoints) > 0:
            print("Failure to register required handles for service {}: {}".format(type(self), broken_endpoints))

            # TODO: Explicit `deregister` is not implemented
            # deregister_id = "foo"
            # self._comm.send(rpc.Message(call="deregister_app", args={'handles': registered_handles}, msg_id="foo"))
            # self._comm.drop_message(deregister_id)
            return False

        # Add any "registered" endpoints to the dispatcher
        for handle in registered_handles:
            dispatcher.register_endpoint(handle, self)
        return True
