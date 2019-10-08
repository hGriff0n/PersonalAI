
# standard imports
import asyncio
import typing
# import uuid

# third-part imports

# local imports
from personal_ai import communication
from personal_ai import dispatcher
from personal_ai import logger
from personal_ai.plugins import plugin
from personal_ai import rpc
from personal_ai.rpc import registration


_REGISTERED_SERVICES: typing.List[typing.Type[plugin.Plugin]] = []
def get_registered_services() -> typing.List[typing.Type[plugin.Plugin]]:
    return _REGISTERED_SERVICES


class Service(plugin.Plugin):
    """
    Definition class which marks all plugins that inherit from it as a service
    Services export rpc endpoints to the wider network, so that other clients can be called

    NOTE: This class should only be inherited on leaf nodes
    TODO: Is there anyway to detect when this isn't happening?

    This class automatically defines `main` to do a "register_app" call, which exports it's endpoints
    To provide "main" service, overload the `run` method
    NOTE: Registration fails if any required endpoint is not registered
    """

    def __init__(self, args: typing.List[str], comm: communication.CommunicationHandler, log: logger.Logger):
        super().__init__(args, comm, log)
        self._registered = False


    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global _REGISTERED_SERVICES
        _REGISTERED_SERVICES.append(cls)
        registration.associate_endpoints_with_service(cls)


    async def main(self) -> bool:
        """
        NOTE: Services should overload `run` to perform any "recurrent" operations

        Automatically performs a `register_app` call on the first pass, initializing the
        Service in the device manager. After registration, the endpoints will automatically be invoked
        Whenever the modality recieves a request for them (on success). Calls `run` all other passes
        """
        if not self._registered:
            self._registered = await self._register()
            return self._registered

        return await self.run()

    async def run(self):
        """
        Entrypoint for any code that app servers need to be run semi-regularly. Defaults to doing nothing.
        """
        await asyncio.sleep(5)
        return True

    async def _register(self):
        """
        Construct and perform the registration calls to the network

        Fails if any required handle is not registered
        """
        endpoints = registration.endpoints_for_class(type(self))
        handles = [handle for handle, _ in endpoints.items()]
        required = [handle for handle, endpoint in endpoints.items() if endpoint.get('required')]

        resp = await self._comm.wait_response(rpc.Message(call="register_app", args={ 'handles': handles }))

        registered_handles = []
        if resp.resp and 'registered' in resp.resp:
            registered_handles.extend(resp.resp['registered'])

        # Check that all required handles are registered (if any)
        # If some handle is required but fails to register, then deregister the whole app
        broken_endpoints = [handle for handle in required if handle not in registered_handles]
        if len(broken_endpoints) > 0:
            # TODO: Explicit `deregister` is not implemented
            # deregister_id = "foo"
            # self._comm.send(rpc.Message(call="deregister_app", args={'handles': registered_handles}, msg_id="foo"))
            # self._comm.drop_message(deregister_id)
            return False

        # Add any "registered" endpoints to the dispatcher
        for handle in registered_handles:
            dispatcher.register_endpoint(handle, self)
        return True
