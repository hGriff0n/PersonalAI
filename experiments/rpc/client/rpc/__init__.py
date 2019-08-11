
import abc
import inspect
import typing


UntypedMessage = typing.Dict[str, typing.Any]


class BaseMessage(object):
    @abc.abstractmethod
    def to_dict(self) -> UntypedMessage:
        """
        Convert this message to a dictionary (for generic storage and transmission)
        """

    @abc.abstractmethod
    def populate_from_dict(self, msg_dict: UntypedMessage) -> bool:
        """
        Populate this message from a given dictionary
        """

    M = typing.TypeVar('M', bound="BaseMessage")
    @classmethod
    def from_dict(kls: typing.Type['BaseMessage.M'], msg_dict: UntypedMessage) -> typing.Optional['BaseMessage.M']:
        obj = kls()
        if not obj.populate_from_dict(msg_dict):
            return None
        return obj


class Message(BaseMessage):

    def __init__(self,
                 msg_id: typing.Optional[str] = None,
                 call: typing.Optional[str] = None,
                 args: typing.Optional[UntypedMessage] = None,
                 resp: typing.Optional[UntypedMessage] = None) -> None:
        self._msg_id = msg_id or ""
        self._call = call or ""
        self._args = args or {}
        self._resp: typing.Optional[UntypedMessage] = resp

    def to_dict(self) -> UntypedMessage:
        ret_dict = {
            'call': self._call,
            'args': self._args,
            'msg_id': self._msg_id,
        }
        if self._resp is not None:
            ret_dict['resp'] = self._resp
        return ret_dict

    def populate_from_dict(self, msg_dict: UntypedMessage) -> bool:
        if ('msg_id' not in msg_dict) or ('call' not in msg_dict) or ('args' not in msg_dict):
            return False

        msg_vals = msg_dict.copy()
        self._msg_id = str(msg_vals.pop('msg_id'))
        self._call = str(msg_vals.pop('call'))
        self._args = dict(msg_vals.pop('args'))

        # `resp` is an optional value, so let's not throw on it
        self._resp = msg_vals.pop('resp', None)

        # If the provided msg def provides more keys than we expect
        # This is an invalid object, so let's return false
        return len(msg_vals) == 0

    @property
    def call(self) -> str:
        return self._call

    @call.setter
    def call(self, val: str) -> None:
        self._call = val

    @property
    def msg_id(self) -> str:
        return self._msg_id

    @property
    def args(self) -> UntypedMessage:
        return self._args

    @property
    def resp(self) -> typing.Optional[UntypedMessage]:
        return self._resp

    @resp.setter
    def resp(self, val: UntypedMessage) -> None:
        self._resp = val


##
## Service decorators
##

# This class enables typing restrictions on the endpoint decorators
# We can't use the actual Plugin base class because of dependencies on communication (which depends on us)
class PluginBase(object):
    pass


__RpcEndpoint = typing.Dict[str, typing.Any]
__ACTIVE_ENDPOINT_REGISTRATION: typing.List[__RpcEndpoint] = []
__REGISTERED_ENDPOINTS: typing.Dict[typing.Type[PluginBase], typing.List[__RpcEndpoint]] = {}


# I think all the types and internal manipulations should move to a module
# The `endpoint` and `service` decorators need to exist in this module
# NOTE: The usage of `Optional` accepts functions that return None or and EndpointResp
# NOTE: While Apps must return something to the server, that is handled with the decorator
EndpointSelfType = typing.TypeVar('EndpointSelfType', bound='PluginBase')
EndpointResp = typing.TypeVar('EndpointResp', bound='BaseMessage')
EndpointRetVal = typing.Coroutine[typing.Any, typing.Any, typing.Optional[EndpointResp]]
NoArgEndpoint = typing.Callable[[EndpointSelfType], EndpointRetVal]
ArgEndpoint = typing.Callable[[EndpointSelfType, BaseMessage.M], EndpointRetVal]
AllEndpointTypes = typing.Union[NoArgEndpoint, ArgEndpoint]
RpcEndpointFnType = typing.Callable[[EndpointSelfType, Message], typing.Coroutine[typing.Any, typing.Any, Message]]


# From the plugin perspective, endpoints take in some subclass of rpc.BaseMessage and return a different subclass (or no-arg/no-ret)
# From the dispatcher perspective, endpoints should take in a rpc.Message and return a rpc.Message
# This decorator automatically inserts the requisite steps to transform the dispatcher function into the plugin function,
# Taking the types from the function signature. It also registers the endpoint for the `register_app` call
# Errors in the endpoint should be communicated by raising an Exception
def endpoint(_func: typing.Optional[AllEndpointTypes] = None,
                 name: typing.Optional[str] = None):
    # Helper function for registring the endpoint
    # This allows for "renaming" the endpoint (ie. the dispatcher calls one function, server calls another)
    def _register_rpc_endpoint(func: AllEndpointTypes, name: typing.Optional[str]) -> None:
        global __ACTIVE_ENDPOINT_REGISTRATION
        __ACTIVE_ENDPOINT_REGISTRATION.append({
            'name': func.__name__,
            'rpc_name': name if name is not None else func.__name__,
        })

    # Create the actual endpoint wrapper for the function
    # This wrapper automatically handles the parsing of the message arguments into the expected messge type
    # And automatically marshalling any responses into the appropriate fields (and handling errors)
    # NOTE: This is explicitly implemented with no type checks as it is not really possible to do this with type checks on
    # For one thing, we determine the type of the arg message from the function signature (using type annotations)
    # Functions can take no args, which makes calling the function impossible as there's 'is_type' check to make this is safe
    def _decorator_no_type_check(func):
        # Extract the ret and arg types for the specific function
        # This enables us to add in automatic casting from the message arguments
        # TODO: Require that the types are subtypes of BaseMessage
        fn_sig = inspect.signature(func)
        ret_ty = fn_sig.return_annotation
        if ret_ty == inspect.Signature.empty:
            ret_ty = None
        try:
            arg_ty = list(fn_sig.parameters.items())[1][1].annotation
        except:
            arg_ty = None

        # Add error reporting and return parsing to the no-arg endpoint function
        async def _call_no_arg(plugin) -> typing.Optional[UntypedMessage]:
            try:
                resp = await func(plugin)
                if resp is not None:
                    return resp.to_dict()
                return None

            except Exception as e:
                return {'error': str(e)}

        # Add error reporting, return parsing, and argument parsing to the arg endpoint function
        async def _call_with_arg(plugin, args: UntypedMessage) -> typing.Optional[UntypedMessage]:
            try:
                typed_args = arg_ty.from_dict(args)
                if typed_args is None:
                    return {'error': "failed to parse arguments as a {}".format(arg_ty)}

                resp = await func(plugin, typed_args)
                if resp is not None:
                    return resp.to_dict()
                return None

            except Exception as e:
                return {'error': str(e)}

        # This is the actual function that get's sent to the rpc registration
        async def _wrapper(self, msg: Message) -> Message:
            if arg_ty is None:
                msg.resp = await _call_no_arg(self)
            else:
                msg.resp = await _call_with_arg(self, msg.args)
            return msg

        # Set the name and docstring of the wrapped function
        # NOTE: functools.wraps also modifies the signature which we actually don't want
        _wrapper.__name__ = func.__name__
        _wrapper.__doc__ = func.__doc__
        return _wrapper

    # Decorator implementation
    # This handles registration and type checking of arguments
    def _decorator(func: AllEndpointTypes) -> RpcEndpointFnType:
        _register_rpc_endpoint(func, name=name)
        return _decorator_no_type_check(func)

    if _func is None:
        return _decorator
    return _decorator(_func)


# Class decorator that maps the registered endpoints to the specific service
# This is done because the class decorators are run **after** the endpoint function decorators
def service(kls: typing.Type[PluginBase]):
    global __ACTIVE_ENDPOINT_REGISTRATION
    global __REGISTERED_ENDPOINTS

    __REGISTERED_ENDPOINTS[kls] = __ACTIVE_ENDPOINT_REGISTRATION
    __ACTIVE_ENDPOINT_REGISTRATION = []
    return kls


def endpoints_for_class(kls: typing.Type[PluginBase]):
    return __REGISTERED_ENDPOINTS.get(kls, [])
