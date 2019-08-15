# standard imports
import typing

# third-part imports

# local imports
import rpc
from rpc import registration


DispatcherFunction = typing.Callable[[rpc.Message], typing.Coroutine[typing.Any, typing.Any, rpc.Message]]
__DISPATCHER: typing.Dict[str, DispatcherFunction]


def get_dispatch_routine(call: str) -> typing.Optional[DispatcherFunction]:
    return __DISPATCHER.get(call)


# NOTE: The endpoint mapping will never overlap (otherwise the server wouldn't have registered the function)
def register_endpoint(endpoint: str, plugin: rpc.PluginBase):
    registered = registration.endpoints_for_class(type(plugin)).get(endpoint)
    if registered is not None:
        __DISPATCHER[endpoint] = getattr(plugin, registered)
