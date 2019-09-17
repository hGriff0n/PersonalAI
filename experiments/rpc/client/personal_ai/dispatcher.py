# standard imports
import typing

# third-part imports

# local imports
from personal_ai import communication
from personal_ai import rpc
from personal_ai.rpc import registration


DispatcherFunction = typing.Callable[[rpc.Message, communication.CommunicationHandler], typing.Coroutine[typing.Any, typing.Any, None]]
__DISPATCHER: typing.Dict[str, DispatcherFunction] = {}


def get_dispatch_routine(call: str) -> typing.Optional[DispatcherFunction]:
    return __DISPATCHER.get(call)


# NOTE: The endpoint mapping will never overlap (otherwise the server wouldn't have registered the function)
def register_endpoint(endpoint: str, plugin: rpc.PluginBase) -> None:
    registered = registration.endpoints_for_class(type(plugin)).get(endpoint)
    if registered is None:
        return None

    endpoint_fn = getattr(plugin, registered['func'])
    async def _endpoint_dispatcher(msg: rpc.Message, comm: communication.CommunicationHandler) -> None:
        comm.write_queue.put(await endpoint_fn(msg))

    __DISPATCHER[endpoint] = _endpoint_dispatcher
