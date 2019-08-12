# standard imports
import typing

# third-part imports

# local imports
from rpc import message

# Plugin typing
class PluginBase(object):
    """
    This class enables typing restrictions on the endpoint decorators
    We can't use the actual Plugin base class because of dependencies on communication (which depends on us)
    """
    pass


# Setup the types of the acceptable endpoint functions (for `@service` and `@endpoint`)
EndpointSelfTypeVar = typing.TypeVar('EndpointSelfTypeVar', bound='PluginBase')
EndpointRespTypeVar = typing.TypeVar('EndpointRespTypeVar', bound='message.BaseMessage')

# NOTE: The usage of `Optional` allows for endpoints to not have a response (ie. return None)
# While Apps currently always return to the server, this can be handled from the dispatcher/decorator
EndpointResponseType = typing.Coroutine[typing.Any, typing.Any, typing.Optional[EndpointRespTypeVar]]

# The two kinds of functions that we allow within `@endpoint` as AppServer endpoints (technically 4)
# 1) No arg endpoint with no response value
# 2) No arg endpoint with a response value
# 3) Arg endpoint with no response value
# 4) Arg endpoint with a response value
EndpointWithNoArgs = typing.Callable[[EndpointSelfTypeVar], EndpointResponseType]
EndpointWithArgs = typing.Callable[[EndpointSelfTypeVar, message.BaseMessage.M], EndpointResponseType]
AllEndpointTypes = typing.Union[EndpointWithArgs, EndpointWithNoArgs]

# The type of the function expected within the dispatcher for endpoints
# NOTE: Conversion from the above functions to this one will be handled within the `@endpoint` decorator
DispatcherEndpointType = typing.Callable[[EndpointSelfTypeVar, message.Message], typing.Coroutine[typing.Any, typing.Any, message.Message]]
